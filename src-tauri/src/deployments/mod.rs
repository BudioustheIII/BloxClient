//! Roblox deployment management, using Roblox's official hosts and manifests.

mod error;
mod extract;
mod install;
mod manifest;
mod model;
mod scan;
mod signature;
mod source;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use reqwest::Client;
use rusqlite::{params, OptionalExtension, Row};

pub use error::DeploymentError;
pub use install::Reporter;
pub use model::{DeploymentInfo, DeploymentOrigin, InstallProgress, InstalledDeployment};

use crate::db::Database;
use crate::error::AppResult;
use crate::roblox::is_player_running;
use crate::settings::DeploymentHost;
use model::{DeploymentStatus, VersionHash};
use scan::{scan_roblox_versions, status_of, PLAYER_EXE};
use signature::{verify_publisher, ROBLOX_PUBLISHER};

pub struct DeploymentManager {
    db: Arc<Database>,
    /// BloxClient's own install folder. The only place it ever deletes from.
    root: PathBuf,
    /// Roblox's versions folders. Strictly read-only.
    roblox_versions: Vec<PathBuf>,
    client: Client,
    active: Mutex<Option<Arc<AtomicBool>>>,
}

/// Marks an install as running for as long as it is alive.
struct ActiveInstall<'a>(&'a DeploymentManager);

impl Drop for ActiveInstall<'_> {
    fn drop(&mut self) {
        *self.0.active.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<InstalledDeployment> {
    let path: String = row.get(1)?;
    Ok(InstalledDeployment {
        version: row.get(0)?,
        status: status_of(Path::new(&path)),
        path,
        origin: DeploymentOrigin::Managed,
        size_bytes: Some(row.get::<_, i64>(2)? as f64),
        installed_at: Some(row.get::<_, i64>(3)? as f64),
    })
}

impl DeploymentManager {
    pub fn new(db: Arc<Database>, root: PathBuf, roblox_versions: Vec<PathBuf>) -> AppResult<Self> {
        fs::create_dir_all(&root)?;
        let manager = Self {
            db,
            root,
            roblox_versions,
            client: source::build_client()?,
            active: Mutex::new(None),
        };
        manager.clear_stale_staging();
        Ok(manager)
    }

    /// Removes staging folders left by a crash or power loss.
    fn clear_stale_staging(&self) {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        for entry in entries.flatten() {
            let is_staging = entry.file_name().to_string_lossy().starts_with(".staging-version-");
            if is_staging && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                if let Err(error) = fs::remove_dir_all(entry.path()) {
                    log::warn!("could not clear old staging folder: {error}");
                }
            }
        }
    }

    pub fn list(&self) -> AppResult<Vec<InstalledDeployment>> {
        let mut deployments = {
            let connection = self.db.connection();
            let mut statement = connection.prepare(
                "SELECT version, install_path, size_bytes, installed_at FROM deployments",
            )?;
            let rows = statement.query_map([], read_row)?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for root in &self.roblox_versions {
            deployments.extend(scan_roblox_versions(root));
        }
        deployments.sort_by(|a, b| {
            b.installed_at
                .unwrap_or(0.0)
                .total_cmp(&a.installed_at.unwrap_or(0.0))
                .then_with(|| a.version.cmp(&b.version))
        });
        Ok(deployments)
    }

    fn managed(&self, version: &VersionHash) -> AppResult<Option<InstalledDeployment>> {
        Ok(self
            .db
            .connection()
            .query_row(
                "SELECT version, install_path, size_bytes, installed_at FROM deployments WHERE version = ?1",
                [version.as_str()],
                read_row,
            )
            .optional()?)
    }

    pub async fn validate(&self, input: &str, choice: DeploymentHost) -> AppResult<DeploymentInfo> {
        let version = VersionHash::parse(input)?;
        let manifests = source::fetch_manifests(&self.client, &version, choice).await?;
        let packages = install::installable(&manifests.packages);

        let installed = self
            .list()?
            .into_iter()
            .filter(|deployment| deployment.version == version.as_str())
            .collect();

        Ok(DeploymentInfo {
            version: version.as_str().to_string(),
            source_host: manifests.host.trim_start_matches("https://").to_string(),
            download_bytes: packages.iter().map(|package| package.zip_size as f64).sum(),
            install_bytes: packages.iter().map(|package| package.raw_size as f64).sum(),
            file_count: manifests.files.len() as u32,
            installed,
        })
    }

    pub async fn install(
        &self,
        input: &str,
        choice: DeploymentHost,
        report: Reporter,
    ) -> AppResult<InstalledDeployment> {
        let version = VersionHash::parse(input)?;
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
            if active.is_some() {
                return Err(DeploymentError::Busy.into());
            }
            *active = Some(Arc::clone(&cancel));
        }
        let _active = ActiveInstall(self);

        let destination = self.root.join(version.as_str());
        if self.managed(&version)?.is_some() || destination.exists() {
            return Err(DeploymentError::AlreadyInstalled.into());
        }

        let manifests = source::fetch_manifests(&self.client, &version, choice).await?;
        let host = manifests.host;
        let staging = install::staging_dir(&self.root, &version);
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }

        let result = async {
            let (files, size) = install::build_verified_tree(
                &self.client,
                &self.root,
                &version,
                choice,
                manifests,
                cancel,
                report,
            )
            .await?;
            fs::rename(&files, &destination)?;
            let recorded = self.db.connection().execute(
                "INSERT INTO deployments (version, install_path, source_host, size_bytes) VALUES (?1, ?2, ?3, ?4)",
                params![
                    version.as_str(),
                    destination.display().to_string(),
                    host.trim_start_matches("https://"),
                    size as i64
                ],
            );
            if let Err(error) = recorded {
                fs::remove_dir_all(&destination)?;
                return Err(error.into());
            }
            AppResult::Ok(())
        }
        .await;

        if staging.exists() {
            if let Err(error) = fs::remove_dir_all(&staging) {
                log::warn!("could not remove staging folder: {error}");
            }
        }
        result?;
        log::info!("installed deployment {}", version.as_str());
        self.managed(&version)?.ok_or_else(|| DeploymentError::NotInstalled.into())
    }

    pub fn cancel_install(&self) {
        if let Some(cancel) = &*self.active.lock().unwrap_or_else(PoisonError::into_inner) {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Only a recorded install directly inside BloxClient's folder can be removed.
    pub fn remove(&self, input: &str) -> AppResult<()> {
        let version = VersionHash::parse(input)?;
        let record = self.managed(&version)?.ok_or(DeploymentError::NotInstalled)?;
        let path = PathBuf::from(&record.path);

        if path.exists() {
            let target = self.checked_managed_dir(&path, &version)?;
            fs::remove_dir_all(&target)?;
        }
        self.db
            .connection()
            .execute("DELETE FROM deployments WHERE version = ?1", [version.as_str()])?;
        log::info!("removed deployment {}", version.as_str());
        Ok(())
    }

    /// Confirms a path is a real version folder directly inside BloxClient's root.
    fn checked_managed_dir(&self, path: &Path, version: &VersionHash) -> AppResult<PathBuf> {
        let root = fs::canonicalize(&self.root)?;
        let target = fs::canonicalize(path)?;
        let is_link = fs::symlink_metadata(path)?.file_type().is_symlink();
        let named_correctly = target.file_name().is_some_and(|name| name == version.as_str());
        if is_link || !named_correctly || target.parent() != Some(root.as_path()) {
            return Err(DeploymentError::UnsafePath.into());
        }
        Ok(target)
    }

    /// Starts the official Player unmodified, after re-checking its signature.
    /// `prepare` runs only for BloxClient's own installs, and a failure there
    /// stops the launch rather than starting Roblox with the wrong settings.
    pub fn launch(
        &self,
        input: &str,
        origin: DeploymentOrigin,
        prepare: impl FnOnce(&Path) -> AppResult<()>,
    ) -> AppResult<()> {
        let version = VersionHash::parse(input)?;
        let deployment = self
            .list()?
            .into_iter()
            .find(|item| {
                item.version == version.as_str() && item.origin == origin && item.status == DeploymentStatus::Ready
            })
            .ok_or(DeploymentError::NotInstalled)?;
        if is_player_running() {
            return Err(DeploymentError::RobloxRunning.into());
        }

        let directory = PathBuf::from(&deployment.path);
        let executable = directory.join(PLAYER_EXE);
        verify_publisher(&executable, ROBLOX_PUBLISHER)?;
        if origin == DeploymentOrigin::Managed {
            prepare(&self.checked_managed_dir(&directory, &version)?)?;
        }
        Command::new(&executable).arg("--app").current_dir(&directory).spawn()?;
        log::info!("launched deployment {}", version.as_str());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    struct Fixture {
        base: PathBuf,
        manager: DeploymentManager,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let base = std::env::temp_dir().join(format!("bloxclient-manager-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(base.join("roblox")).unwrap();
            let db = Arc::new(Database::in_memory().unwrap());
            let manager = DeploymentManager::new(db, base.join("managed"), vec![base.join("roblox")]).unwrap();
            Self { base, manager }
        }

        fn record(&self, version: &str, path: &Path) {
            self.manager
                .db
                .connection()
                .execute(
                    "INSERT INTO deployments (version, install_path, source_host, size_bytes) VALUES (?1, ?2, 'test', 1)",
                    params![version, path.display().to_string()],
                )
                .unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    const VERSION: &str = "version-aaaaaaaaaaaaaaaa";

    #[test]
    fn removes_only_the_managed_folder() {
        let fixture = Fixture::new("remove");
        let install = fixture.manager.root.join(VERSION);
        let neighbor = fixture.manager.root.join("version-bbbbbbbbbbbbbbbb");
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&neighbor).unwrap();
        fs::write(install.join("file"), b"x").unwrap();
        fixture.record(VERSION, &install);

        fixture.manager.remove(VERSION).unwrap();
        assert!(!install.exists());
        assert!(neighbor.exists());
        assert!(fixture.manager.managed(&VersionHash::parse(VERSION).unwrap()).unwrap().is_none());
    }

    #[test]
    fn refuses_records_pointing_outside_the_managed_folder() {
        let fixture = Fixture::new("outside");
        let roblox_install = fixture.base.join("roblox").join(VERSION);
        fs::create_dir_all(&roblox_install).unwrap();
        fs::write(roblox_install.join("RobloxPlayerBeta.exe"), b"x").unwrap();
        fixture.record(VERSION, &roblox_install);

        let result = fixture.manager.remove(VERSION);
        assert!(matches!(result, Err(AppError::Deployment(DeploymentError::UnsafePath))));
        assert!(roblox_install.join("RobloxPlayerBeta.exe").exists());
    }

    #[test]
    fn refuses_wrongly_named_folder_inside_root() {
        let fixture = Fixture::new("wrongname");
        let other = fixture.manager.root.join("version-cccccccccccccccc");
        fs::create_dir_all(&other).unwrap();
        fixture.record(VERSION, &other);

        let result = fixture.manager.remove(VERSION);
        assert!(matches!(result, Err(AppError::Deployment(DeploymentError::UnsafePath))));
        assert!(other.exists());
    }

    #[test]
    fn roblox_installs_cannot_be_removed() {
        let fixture = Fixture::new("robloxowned");
        let install = fixture.base.join("roblox").join(VERSION);
        fs::create_dir_all(&install).unwrap();
        fs::write(install.join("RobloxPlayerBeta.exe"), b"x").unwrap();

        assert_eq!(fixture.manager.list().unwrap().len(), 1);
        let result = fixture.manager.remove(VERSION);
        assert!(matches!(result, Err(AppError::Deployment(DeploymentError::NotInstalled))));
        assert!(install.exists());
    }

    #[test]
    fn stale_staging_is_cleared_but_installs_are_kept() {
        let fixture = Fixture::new("staging");
        let stale = fixture.manager.root.join(format!(".staging-{VERSION}"));
        let install = fixture.manager.root.join(VERSION);
        fs::create_dir_all(stale.join("files")).unwrap();
        fs::create_dir_all(&install).unwrap();

        fixture.manager.clear_stale_staging();
        assert!(!stale.exists());
        assert!(install.exists());
    }

    #[test]
    fn launch_refuses_unsigned_or_missing_installs() {
        let fixture = Fixture::new("launch");
        let install = fixture.manager.root.join(VERSION);
        fs::create_dir_all(install.join("content")).unwrap();
        for file in [PLAYER_EXE, "RobloxPlayerBeta.dll", "AppSettings.xml"] {
            fs::write(install.join(file), b"not signed").unwrap();
        }
        fixture.record(VERSION, &install);

        // Skipped when a real Player is open, which reports RobloxRunning first.
        if !is_player_running() {
            let result = fixture.manager.launch(VERSION, DeploymentOrigin::Managed, |_| {
                panic!("settings must not be written for an unsigned install")
            });
            assert!(matches!(result, Err(AppError::Deployment(DeploymentError::Signature(_)))));
        }
        let missing = fixture
            .manager
            .launch("version-dddddddddddddddd", DeploymentOrigin::Managed, |_| Ok(()));
        assert!(matches!(missing, Err(AppError::Deployment(DeploymentError::NotInstalled))));
    }

    #[test]
    fn settings_target_must_be_inside_the_managed_folder() {
        let fixture = Fixture::new("target");
        let version = VersionHash::parse(VERSION).unwrap();
        let inside = fixture.manager.root.join(VERSION);
        let outside = fixture.base.join("roblox").join(VERSION);
        fs::create_dir_all(&inside).unwrap();
        fs::create_dir_all(&outside).unwrap();

        assert!(fixture.manager.checked_managed_dir(&inside, &version).is_ok());
        assert!(matches!(
            fixture.manager.checked_managed_dir(&outside, &version),
            Err(AppError::Deployment(DeploymentError::UnsafePath))
        ));
    }
}
