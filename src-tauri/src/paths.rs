use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::error::AppResult;

const DATABASE_FILE: &str = "bloxclient.db";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub database: PathBuf,
    /// BloxClient's isolated deployment installs.
    pub deployments: PathBuf,
    /// Roblox's own per-user and system-wide version folders. Read-only.
    pub roblox_versions: Vec<PathBuf>,
    /// Roblox's own log folder. Read-only.
    pub roblox_logs: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let (data_dir, log_dir, deployments) = match debug_data_override() {
            Some(dir) => (dir.clone(), dir.join("logs"), dir.join("Deployments")),
            None => (
                app.path().app_data_dir()?,
                app.path().app_log_dir()?,
                // Large and machine-specific, so it stays out of the roaming profile.
                app.path().app_local_data_dir()?.join("Deployments"),
            ),
        };
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&log_dir)?;

        let mut roblox_versions = vec![app.path().local_data_dir()?.join("Roblox").join("Versions")];
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            roblox_versions.push(PathBuf::from(program_files).join("Roblox").join("Versions"));
        }

        // Development runs can point at fixture logs. Ignored in release builds.
        let test_logs = cfg!(debug_assertions)
            .then(|| std::env::var_os("BLOXCLIENT_TEST_ROBLOX_LOGS"))
            .flatten();
        let roblox_logs = match test_logs {
            Some(folder) => PathBuf::from(folder),
            None => app.path().local_data_dir()?.join("Roblox").join("logs"),
        };

        Ok(Self {
            database: data_dir.join(DATABASE_FILE),
            roblox_versions,
            roblox_logs,
            deployments,
            data_dir,
            log_dir,
        })
    }
}

/// Lets development runs use a throwaway data folder. Ignored in release builds.
pub fn debug_data_override() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        std::env::var_os("BLOXCLIENT_DATA_DIR").map(PathBuf::from)
    } else {
        None
    }
}
