use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::roblox::{
    history, install, DiscoverFeed, ExperienceDetail, LaunchTarget, PlaySession, RecentExperience, RobloxInstall,
    SearchPage,
};
use crate::state::AppState;

const RECENT_LIMIT: u32 = 24;

#[tauri::command]
pub async fn get_discover_feed(state: State<'_, AppState>, refresh: bool) -> AppResult<DiscoverFeed> {
    Ok(state.catalog.feed(refresh).await?)
}

#[tauri::command]
pub async fn search_experiences(
    state: State<'_, AppState>,
    query: String,
    page_token: Option<String>,
) -> AppResult<SearchPage> {
    Ok(state.catalog.search(&query, page_token.as_deref()).await?)
}

#[tauri::command]
pub async fn get_experience(state: State<'_, AppState>, universe_id: u64) -> AppResult<ExperienceDetail> {
    Ok(state.catalog.detail(universe_id).await?)
}

#[tauri::command]
pub async fn get_roblox_install() -> AppResult<RobloxInstall> {
    Ok(tauri::async_runtime::spawn_blocking(install::detect).await?)
}

#[tauri::command]
pub async fn launch_experience(
    app: AppHandle,
    state: State<'_, AppState>,
    target: LaunchTarget,
    force: bool,
) -> AppResult<PlaySession> {
    let sessions = state.sessions.clone();
    tauri::async_runtime::spawn_blocking(move || sessions.launch(&app, target, force)).await?
}

#[tauri::command]
pub fn get_play_session(state: State<'_, AppState>) -> Option<PlaySession> {
    state.sessions.current()
}

#[tauri::command]
pub fn list_recent_experiences(state: State<'_, AppState>) -> AppResult<Vec<RecentExperience>> {
    history::list(&state.db, RECENT_LIMIT)
}

/// Imports games played outside BloxClient from Roblox's local logs.
#[tauri::command]
pub async fn sync_play_history(state: State<'_, AppState>) -> AppResult<Vec<RecentExperience>> {
    state.play_history.sync(&state.catalog).await?;
    history::list(&state.db, RECENT_LIMIT)
}

#[tauri::command]
pub fn clear_recent_experiences(state: State<'_, AppState>) -> AppResult<()> {
    history::clear(&state.db)
}
