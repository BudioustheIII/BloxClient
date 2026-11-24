//! Package extraction and integrity checks. Blocking; run off the async runtime.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use md5::{Digest, Md5};
use zip::ZipArchive;

use super::error::DeploymentError;
use super::manifest::{safe_relative, FileManifest};
use crate::error::{AppError, AppResult};

/// Listed in the manifest but only used by the WebView2 runtime installer
/// package, which BloxClient does not install.
const OPTIONAL_FILES: &[&str] = &["microsoftedgewebview2setup.exe"];
const BUFFER_SIZE: usize = 128 * 1024;

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), DeploymentError> {
    if cancel.load(Ordering::Relaxed) {
        Err(DeploymentError::Cancelled)
    } else {
        Ok(())
    }
}

fn integrity(detail: impl Into<String>) -> AppError {
    DeploymentError::Integrity(detail.into()).into()
}

/// Copies at most `limit` bytes, hashing as it goes.
fn copy_hashed(
    reader: &mut impl Read,
    writer: &mut impl Write,
    limit: u64,
    cancel: &AtomicBool,
    on_bytes: &mut dyn FnMut(u64),
) -> AppResult<(u64, String)> {
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut hasher = Md5::new();
    let mut total = 0u64;
    loop {
        check_cancel(cancel)?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok((total, to_hex(&hasher.finalize())));
        }
        total += read as u64;
        if total > limit {
            return Err(integrity("archive entry is larger than declared"));
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read])?;
        on_bytes(read as u64);
    }
}

/// Extracts one package into `files_root`. Entries go to a scratch folder
/// first, and only move into place once the whole package matches the manifest.
pub fn extract_package(
    archive_path: &Path,
    scratch: &Path,
    files_root: &Path,
    manifest: &FileManifest,
    placed: &mut HashSet<String>,
    cancel: &AtomicBool,
    on_bytes: &mut dyn FnMut(u64),
) -> AppResult<()> {
    let package = archive_path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let mut archive = ZipArchive::new(BufReader::new(File::open(archive_path)?))
        .map_err(|error| integrity(format!("{package}: {error}")))?;

    fs::create_dir_all(scratch)?;
    let mut extracted: Vec<(String, String)> = Vec::with_capacity(archive.len());

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| integrity(format!("{package}: {error}")))?;
        let raw_name = entry
            .name()
            .map_err(|error| integrity(format!("{package}: {error}")))?
            .into_owned();
        if raw_name.ends_with('/') || raw_name.ends_with('\\') {
            continue;
        }
        if entry.is_symlink() {
            return Err(integrity(format!("{package}: contains a link")));
        }
        let name = safe_relative(&raw_name).ok_or_else(|| integrity(format!("{package}: unsafe entry path")))?;
        let declared = entry.size();

        let scratch_file = scratch.join(extracted.len().to_string());
        let mut writer = BufWriter::new(File::create(&scratch_file)?);
        let (written, md5) = copy_hashed(&mut entry, &mut writer, declared, cancel, on_bytes)?;
        writer.flush()?;
        if written != declared {
            return Err(integrity(format!("{package}: truncated entry")));
        }
        extracted.push((name, md5));
    }

    if !extracted.is_empty() {
        let root = manifest
            .resolve_root(&extracted, placed)
            .ok_or_else(|| integrity(format!("{package}: contents don't match the file manifest")))?;

        for (index, (name, _)) in extracted.iter().enumerate() {
            let key = format!("{root}{}", name.to_lowercase());
            let entry = manifest.get(&key).ok_or_else(|| integrity("file left the manifest"))?;
            let destination = files_root.join(&entry.path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(scratch.join(index.to_string()), &destination)?;
            placed.insert(key);
        }
    }
    fs::remove_dir_all(scratch)?;
    Ok(())
}

/// Re-hashes every installed file. Returns the installed size in bytes.
pub fn verify_tree(
    root: &Path,
    manifest: &FileManifest,
    cancel: &AtomicBool,
    on_bytes: &mut dyn FnMut(u64),
) -> AppResult<u64> {
    let mut total = 0;
    for entry in manifest.entries() {
        let path = root.join(&entry.path);
        if !path.exists() && OPTIONAL_FILES.contains(&entry.path.to_lowercase().as_str()) {
            continue;
        }
        let mut file = File::open(&path).map_err(|_| integrity(format!("missing {}", entry.path)))?;
        let (size, md5) = copy_hashed(&mut file, &mut std::io::sink(), u64::MAX, cancel, on_bytes)?;
        if md5 != entry.md5 {
            return Err(integrity(format!("hash mismatch for {}", entry.path)));
        }
        total += size;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU32;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    static NEXT_DIR: AtomicU32 = AtomicU32::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("bloxclient-extract-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn md5_of(data: &[u8]) -> String {
        to_hex(&Md5::digest(data))
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        for (name, data) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    fn manifest(files: &[(&str, &[u8])]) -> FileManifest {
        let mut text = format!("RobloxPlayerBeta.exe\n{}\n", md5_of(b"player"));
        for (path, data) in files {
            text.push_str(&format!("{path}\n{}\n", md5_of(data)));
        }
        FileManifest::parse(&text).unwrap()
    }

    fn extract(dir: &TestDir, manifest: &FileManifest, entries: &[(&str, &[u8])]) -> AppResult<()> {
        let archive = dir.0.join("package.zip");
        write_zip(&archive, entries);
        extract_package(
            &archive,
            &dir.0.join("scratch"),
            &dir.0.join("files"),
            manifest,
            &mut HashSet::new(),
            &AtomicBool::new(false),
            &mut |_| {},
        )
    }

    #[test]
    fn extracts_into_the_manifest_location() {
        let dir = TestDir::new();
        let manifest = manifest(&[(r"content\sounds\a.ogg", b"sound"), (r"content\sounds\sub\b.ogg", b"more")]);
        extract(&dir, &manifest, &[("a.ogg", b"sound"), (r"sub\b.ogg", b"more")]).unwrap();
        assert_eq!(fs::read(dir.0.join("files/content/sounds/a.ogg")).unwrap(), b"sound");
        assert_eq!(fs::read(dir.0.join("files/content/sounds/sub/b.ogg")).unwrap(), b"more");
        assert!(!dir.0.join("scratch").exists());
    }

    #[test]
    fn rejects_path_traversal() {
        let dir = TestDir::new();
        let manifest = manifest(&[("a.txt", b"x")]);
        let result = extract(&dir, &manifest, &[("../../evil.txt", b"x")]);
        assert!(matches!(result, Err(AppError::Deployment(DeploymentError::Integrity(_)))));
        assert!(!dir.0.parent().unwrap().join("evil.txt").exists());
    }

    #[test]
    fn tampered_package_places_nothing() {
        let dir = TestDir::new();
        let manifest = manifest(&[(r"content\a.txt", b"good"), (r"content\b.txt", b"good2")]);
        let result = extract(&dir, &manifest, &[("a.txt", b"good"), ("b.txt", b"evil")]);
        assert!(matches!(result, Err(AppError::Deployment(DeploymentError::Integrity(_)))));
        assert!(!dir.0.join("files/content/a.txt").exists());
    }

    #[test]
    fn cancel_stops_extraction() {
        let dir = TestDir::new();
        let manifest = manifest(&[("a.txt", b"x")]);
        let archive = dir.0.join("package.zip");
        write_zip(&archive, &[("a.txt", b"x")]);
        let result = extract_package(
            &archive,
            &dir.0.join("scratch"),
            &dir.0.join("files"),
            &manifest,
            &mut HashSet::new(),
            &AtomicBool::new(true),
            &mut |_| {},
        );
        assert!(matches!(result, Err(AppError::Deployment(DeploymentError::Cancelled))));
    }

    #[test]
    fn verify_detects_changes_and_missing_files() {
        let dir = TestDir::new();
        let manifest = manifest(&[("a.txt", b"data")]);
        let cancel = AtomicBool::new(false);
        fs::write(dir.0.join("RobloxPlayerBeta.exe"), b"player").unwrap();
        fs::write(dir.0.join("a.txt"), b"data").unwrap();
        assert_eq!(verify_tree(&dir.0, &manifest, &cancel, &mut |_| {}).unwrap(), 10);

        fs::write(dir.0.join("a.txt"), b"changed").unwrap();
        assert!(verify_tree(&dir.0, &manifest, &cancel, &mut |_| {}).is_err());

        fs::remove_file(dir.0.join("a.txt")).unwrap();
        assert!(verify_tree(&dir.0, &manifest, &cancel, &mut |_| {}).is_err());
    }
}
