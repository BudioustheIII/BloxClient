#[derive(Debug, thiserror::Error)]
pub enum DeploymentError {
    #[error("version is invalid or not hosted")]
    Unavailable,
    #[error("network error: {0}")]
    Network(String),
    #[error("unreadable manifest: {0}")]
    Manifest(String),
    #[error("integrity check failed: {0}")]
    Integrity(String),
    #[error("signature check failed: {0}")]
    Signature(String),
    #[error("install cancelled")]
    Cancelled,
    #[error("another install is running")]
    Busy,
    #[error("deployment is already installed")]
    AlreadyInstalled,
    #[error("deployment is not installed")]
    NotInstalled,
    #[error("Roblox is already running")]
    RobloxRunning,
    #[error("not enough disk space: need {needed} bytes, have {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    #[error("refused to change a path outside the deployments folder")]
    UnsafePath,
}

impl DeploymentError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable => "deployment_unavailable",
            Self::Network(_) => "deployment_network",
            Self::Manifest(_) => "deployment_manifest",
            Self::Integrity(_) => "deployment_integrity",
            Self::Signature(_) => "deployment_signature",
            Self::Cancelled => "deployment_cancelled",
            Self::Busy => "deployment_busy",
            Self::AlreadyInstalled => "deployment_installed",
            Self::NotInstalled => "deployment_missing",
            Self::RobloxRunning => "roblox_running",
            Self::InsufficientSpace { .. } => "deployment_space",
            Self::UnsafePath => "deployment_path",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Unavailable => {
                "Invalid or unavailable Roblox version hash. Please provide a valid version hash.".into()
            }
            Self::Network(_) => {
                "Roblox's download servers couldn't be reached. Check your connection and try again.".into()
            }
            Self::Manifest(_) => "Roblox's file list for this version couldn't be read.".into(),
            Self::Integrity(_) => {
                "A downloaded file didn't match Roblox's published hash, so nothing was installed.".into()
            }
            Self::Signature(_) => {
                "This version isn't validly signed by Roblox Corporation, so it wasn't accepted.".into()
            }
            Self::Cancelled => "Download cancelled. Nothing was installed.".into(),
            Self::Busy => "Another version is already downloading.".into(),
            Self::AlreadyInstalled => "This version is already installed.".into(),
            Self::NotInstalled => "This version is no longer installed.".into(),
            Self::RobloxRunning => "Roblox is already running. Close it before launching another version.".into(),
            Self::InsufficientSpace { needed, .. } => format!(
                "Not enough disk space. This version needs about {} MB free.",
                needed / (1024 * 1024)
            ),
            Self::UnsafePath => "BloxClient refused to change a folder it doesn't manage.".into(),
        }
    }
}
