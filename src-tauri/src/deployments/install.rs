//! The install pipeline: download, verify, extract, verify again, then move
//! into place. All work happens in a staging folder, so a failed or cancelled
//! install leaves nothing behind and nothing downloaded is ever executed.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use md5::{Digest, Md5};
use reqwest::Client;
use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

use super::error::DeploymentError;
use super::extract::{extract_package, to_hex, verify_tree};
use super::manifest::Package;
use super::model::{InstallProgress, InstallStage, VersionHash};
use super::signature::{verify_publisher, MICROSOFT_PUBLISHER, ROBLOX_PUBLISHER};
use super::source::{hosts_for, network, package_url, Manifests};
use crate::error::{AppError, AppResult};
use crate::settings::DeploymentHost;

const APP_SETTINGS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<Settings>\r\n\t<ContentFolder>content</ContentFolder>\r\n\t<BaseUrl>http://www.roblox.com</BaseUrl>\r\n</Settings>\r\n";
/// Not needed to run the Player. The first would only re-install Roblox.
const SKIPPED_PACKAGES: &[&str] = &["RobloxPlayerInstaller.exe", "WebView2RuntimeInstaller.zip"];
const SIGNED_FILES: &[(&str, &str)] = &[
    ("RobloxPlayerBeta.exe", ROBLOX_PUBLISHER),
    ("RobloxPlayerBeta.dll", ROBLOX_PUBLISHER),
    ("RobloxCrashHandler.exe", ROBLOX_PUBLISHER),
    ("WebView2Loader.dll", MICROSOFT_PUBLISHER),
];
const MAX_ATTEMPTS: usize = 3;
const REPORT_INTERVAL: Duration = Duration::from_millis(200);
const FREE_SPACE_MARGIN: u64 = 256 * 1024 * 1024;

pub type Reporter = Arc<dyn Fn(InstallProgress) + Send + Sync>;

pub fn staging_dir(root: &Path, version: &VersionHash) -> PathBuf {
    root.join(format!(".staging-{}", version.as_str()))
}

pub fn installable(packages: &[Package]) -> Vec<Package> {
    packages
        .iter()
        .filter(|package| package.name.ends_with(".zip") && !SKIPPED_PACKAGES.contains(&package.name.as_str()))
        .cloned()
        .collect()
}

/// Rate-limits progress events so the UI isn't flooded.
struct Progress {
    version: String,
    stage: InstallStage,
    done: u64,
    total: u64,
    last: Instant,
    report: Reporter,
}

impl Progress {
    fn new(version: &VersionHash, report: Reporter) -> Self {
        Self {
            version: version.as_str().to_string(),
            stage: InstallStage::Preparing,
            done: 0,
            total: 0,
            last: Instant::now(),
            report,
        }
    }

    fn start(&mut self, stage: InstallStage, total: u64) {
        self.stage = stage;
        self.done = 0;
        self.total = total;
        self.emit();
    }

    fn set(&mut self, done: u64) {
        self.done = done;
        if self.last.elapsed() >= REPORT_INTERVAL {
            self.emit();
        }
    }

    fn add(&mut self, bytes: u64) {
        self.set(self.done + bytes);
    }

    fn emit(&mut self) {
        self.last = Instant::now();
        (self.report)(InstallProgress {
            version: self.version.clone(),
            stage: self.stage,
            done_bytes: self.done as f64,
            total_bytes: self.total as f64,
        });
    }
}

fn free_space(path: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut available = 0u64;
    // SAFETY: `wide` is NUL-terminated and `available` outlives the call.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, std::ptr::null_mut(), std::ptr::null_mut()) };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(available)
    }
}

async fn download_once(
    client: &Client,
    url: &str,
    package: &Package,
    destination: &Path,
    cancel: &AtomicBool,
    progress: &mut Progress,
    base: u64,
) -> AppResult<()> {
    let mut response = client.get(url).send().await.map_err(network)?;
    if !response.status().is_success() {
        return Err(network(format!("HTTP {}", response.status())).into());
    }

    let mut writer = BufWriter::new(File::create(destination)?);
    let mut hasher = Md5::new();
    let mut written = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if cancel.load(Ordering::Relaxed) {
            return Err(DeploymentError::Cancelled.into());
        }
        written += chunk.len() as u64;
        if written > package.zip_size {
            return Err(DeploymentError::Integrity(format!("{} is larger than published", package.name)).into());
        }
        hasher.update(&chunk);
        writer.write_all(&chunk)?;
        progress.set(base + written);
    }
    writer.flush()?;

    if written != package.zip_size {
        return Err(DeploymentError::Integrity(format!("{} is incomplete", package.name)).into());
    }
    if to_hex(&hasher.finalize()) != package.md5 {
        return Err(DeploymentError::Integrity(format!("{} hash mismatch", package.name)).into());
    }
    Ok(())
}

/// Retries transient failures, moving to the next official host each time.
async fn download(
    client: &Client,
    hosts: &[&str],
    version: &VersionHash,
    package: &Package,
    destination: &Path,
    cancel: &AtomicBool,
    progress: &mut Progress,
) -> AppResult<()> {
    let base = progress.done;
    let mut last_error = None;
    for attempt in 0..MAX_ATTEMPTS {
        let host = hosts[attempt % hosts.len()];
        let url = package_url(host, version, &package.name);
        match download_once(client, &url, package, destination, cancel, progress, base).await {
            Ok(()) => return Ok(()),
            Err(error @ AppError::Deployment(DeploymentError::Cancelled)) => return Err(error),
            Err(error) => {
                log::warn!("download of {} from {host} failed: {error}", package.name);
                progress.set(base);
                last_error = Some(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| network("no download attempts").into()))
}

/// Runs the pipeline up to a fully verified tree at `<staging>/files`.
/// Returns that path and the installed size. The caller moves it into place.
pub async fn build_verified_tree(
    client: &Client,
    root: &Path,
    version: &VersionHash,
    choice: DeploymentHost,
    manifests: Manifests,
    cancel: Arc<AtomicBool>,
    report: Reporter,
) -> AppResult<(PathBuf, u64)> {
    let packages = installable(&manifests.packages);
    let download_total: u64 = packages.iter().map(|package| package.zip_size).sum();
    let raw_total: u64 = packages.iter().map(|package| package.raw_size).sum();

    let needed = download_total + raw_total + FREE_SPACE_MARGIN;
    let available = free_space(root)?;
    if available < needed {
        return Err(DeploymentError::InsufficientSpace { needed, available }.into());
    }

    let staging = staging_dir(root, version);
    let archives = staging.join("packages");
    let files_root = staging.join("files");
    fs::create_dir_all(&archives)?;
    fs::create_dir_all(&files_root)?;

    let mut progress = Progress::new(version, report);
    progress.start(InstallStage::Downloading, download_total);
    let hosts = hosts_for(choice);
    for package in &packages {
        download(client, hosts, version, package, &archives.join(&package.name), &cancel, &mut progress).await?;
    }

    let files = files_root.clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<(PathBuf, u64)> {
        let manifest = manifests.files;
        let scratch = staging.join("scratch");
        let mut placed = HashSet::new();

        progress.start(InstallStage::Extracting, raw_total);
        for package in &packages {
            let archive = archives.join(&package.name);
            extract_package(&archive, &scratch, &files, &manifest, &mut placed, &cancel, &mut |bytes| {
                progress.add(bytes)
            })?;
            fs::remove_file(&archive)?;
        }
        fs::write(files.join("AppSettings.xml"), APP_SETTINGS)?;

        progress.start(InstallStage::Verifying, raw_total);
        let size = verify_tree(&files, &manifest, &cancel, &mut |bytes| progress.add(bytes))?;
        for (file, publisher) in SIGNED_FILES {
            verify_publisher(&files.join(file), publisher)?;
        }
        Ok((files, size))
    })
    .await?
}
