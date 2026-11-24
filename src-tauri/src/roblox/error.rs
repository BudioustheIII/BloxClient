#[derive(Debug, thiserror::Error)]
pub enum RobloxError {
    #[error("network error: {0}")]
    Network(String),
    #[error("rate limited by Roblox")]
    RateLimited,
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
    #[error("experience not found")]
    NotFound,
    #[error("invalid search: {0}")]
    InvalidSearch(&'static str),
    #[error("no program is registered for roblox:// links")]
    NotInstalled,
    #[error("the registered Roblox launcher is missing: {0}")]
    LauncherMissing(String),
    #[error("Roblox is already running")]
    AlreadyRunning,
    #[error("a play session is already active")]
    SessionActive,
    #[error("invalid username")]
    InvalidUsername,
    #[error("user not found")]
    UserNotFound,
    #[error("person already added")]
    AlreadyAdded,
    #[error("list limit reached")]
    ListFull,
    #[error("no local profile selected")]
    NoProfile,
}

impl RobloxError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Network(_) => "roblox_network",
            Self::RateLimited => "roblox_rate_limited",
            Self::UnexpectedResponse(_) => "roblox_response",
            Self::NotFound => "roblox_not_found",
            Self::InvalidSearch(_) => "roblox_invalid_search",
            Self::NotInstalled => "roblox_not_installed",
            Self::LauncherMissing(_) => "roblox_launcher_missing",
            Self::AlreadyRunning => "roblox_already_running",
            Self::SessionActive => "roblox_session_active",
            Self::InvalidUsername => "roblox_invalid_username",
            Self::UserNotFound => "roblox_user_not_found",
            Self::AlreadyAdded => "roblox_already_added",
            Self::ListFull => "roblox_list_full",
            Self::NoProfile => "roblox_no_profile",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Network(_) => "Roblox couldn't be reached. Check your connection and try again.".into(),
            Self::RateLimited => "Roblox is limiting requests right now. Wait a moment and try again.".into(),
            Self::UnexpectedResponse(_) => {
                "Roblox sent data BloxClient couldn't read. This service may have changed.".into()
            }
            Self::NotFound => "This experience doesn't exist or isn't available.".into(),
            Self::InvalidSearch(reason) => format!("Search not run: {reason}."),
            Self::NotInstalled => "Roblox isn't installed. Install the Roblox Player, then try again.".into(),
            Self::LauncherMissing(_) => {
                "The program set to open Roblox games is missing. Reinstall Roblox or your launcher.".into()
            }
            Self::AlreadyRunning => "Roblox is already running.".into(),
            Self::SessionActive => "A game is already starting or running.".into(),
            Self::InvalidUsername => {
                "Roblox usernames are 3 to 20 characters, using letters, numbers and underscores.".into()
            }
            Self::UserNotFound => "No Roblox user has that username.".into(),
            Self::AlreadyAdded => "That person is already on this list.".into(),
            Self::ListFull => "This list is full. Remove someone to add another person.".into(),
            Self::NoProfile => "Add your Roblox username in Accounts first.".into(),
        }
    }
}
