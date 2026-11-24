use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::auth::AuthError;
use crate::deployments::DeploymentError;
use crate::fflags::FlagError;
use crate::library::LibraryError;
use crate::roblox::RobloxError;

/// Backend error. Full detail goes to the log; the frontend receives a stable
/// code and a message written for the person using the app.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("database schema {found} is newer than supported schema {supported}")]
    DatabaseTooNew { found: i64, supported: i64 },
    #[error("invalid settings: {0}")]
    InvalidSettings(String),
    #[error("startup entry error: {0}")]
    Autostart(std::io::Error),
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("window system error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("could not open folder: {0}")]
    Opener(#[from] tauri_plugin_opener::Error),
    #[error("deployment error: {0}")]
    Deployment(#[from] DeploymentError),
    #[error("Roblox error: {0}")]
    Roblox(#[from] RobloxError),
    #[error("sign-in error: {0}")]
    Auth(#[from] AuthError),
    #[error("encrypted storage error: {0}")]
    Vault(String),
    #[error("FFlag error: {0}")]
    FFlags(#[from] FlagError),
    #[error("library error: {0}")]
    Library(#[from] LibraryError),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::DatabaseTooNew { .. } => "database_too_new",
            Self::InvalidSettings(_) => "invalid_settings",
            Self::Autostart(_) => "autostart",
            Self::Io(_) => "io",
            Self::Serialization(_) => "serialization",
            Self::Tauri(_) => "window",
            Self::Opener(_) => "opener",
            Self::Deployment(error) => error.code(),
            Self::Roblox(error) => error.code(),
            Self::Auth(error) => error.code(),
            Self::Vault(_) => "vault",
            Self::FFlags(error) => error.code(),
            Self::Library(error) => error.code(),
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Database(_) => {
                "BloxClient couldn't read or write its local database. Restart the app, and check the logs if this continues.".into()
            }
            Self::DatabaseTooNew { .. } => {
                "This data was created by a newer version of BloxClient. Install the latest version to open it.".into()
            }
            Self::InvalidSettings(detail) => format!("That setting wasn't saved: {detail}."),
            Self::Autostart(_) => {
                "Windows didn't allow the startup entry to change. Check Startup apps in Windows Settings.".into()
            }
            Self::Io(_) => "BloxClient couldn't access its data folder. Check that the drive is available.".into(),
            Self::Serialization(_) => "BloxClient found unreadable local data. Check the logs for details.".into(),
            Self::Tauri(_) => "The window system rejected that action. Try again.".into(),
            Self::Opener(_) => "Windows couldn't open that folder.".into(),
            Self::Deployment(error) => error.user_message(),
            Self::Roblox(error) => error.user_message(),
            Self::Auth(error) => error.user_message(),
            Self::Vault(_) => "Windows couldn't encrypt or read your private BloxClient data.".into(),
            Self::FFlags(error) => error.user_message(),
            Self::Library(error) => error.user_message(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        log::error!("command failed: {self}");
        let mut value = serializer.serialize_struct("AppError", 2)?;
        value.serialize_field("code", self.code())?;
        value.serialize_field("message", &self.user_message())?;
        value.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
