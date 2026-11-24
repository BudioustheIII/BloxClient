use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::auth::AuthManager;
use crate::db::Database;
use crate::deployments::DeploymentManager;
use crate::fflags::FlagManager;
use crate::library::Library;
use crate::paths::AppPaths;
use crate::performance::PerformanceMonitor;
use crate::roblox::{Catalog, PeopleManager, PlayHistorySync, SessionManager};
use crate::settings::SettingsStore;

pub struct AppState {
    pub db: Arc<Database>,
    pub settings: SettingsStore,
    pub deployments: Arc<DeploymentManager>,
    pub fflags: Arc<FlagManager>,
    pub library: Library,
    pub people: PeopleManager,
    pub performance: Arc<PerformanceMonitor>,
    pub catalog: Catalog,
    pub sessions: Arc<SessionManager>,
    pub play_history: PlayHistorySync,
    pub auth: Arc<AuthManager>,
    pub paths: AppPaths,
    /// Set once the main window has received its launch visibility.
    pub revealed: AtomicBool,
}
