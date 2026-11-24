//! Roblox integration: discovery through public APIs, launching through the
//! standard `roblox://` link, and process tracking by name.

mod api;
mod catalog;
mod error;
pub mod history;
pub mod install;
mod http;
mod model;
mod people;
mod people_api;
mod play_sync;
mod player_logs;
mod process;
mod session;
mod visibility;

pub use catalog::Catalog;
pub use error::RobloxError;
pub use model::{
    DiscoverFeed, ExperienceDetail, LaunchTarget, PlaySession, RecentExperience, RobloxInstall, SearchPage,
};
pub use people::{PeopleManager, PeopleOverview};
pub use play_sync::PlayHistorySync;
pub use process::is_player_running;
pub use session::SessionManager;
pub use visibility::spawn_player_watcher;
