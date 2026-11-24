use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppResult;
use crate::roblox::{PeopleOverview, RobloxError};
use crate::state::AppState;

#[tauri::command]
pub async fn get_people(state: State<'_, AppState>) -> AppResult<PeopleOverview> {
    state.people.overview().await
}

#[tauri::command]
pub async fn add_local_profile(state: State<'_, AppState>, username: String) -> AppResult<PeopleOverview> {
    state.people.add_profile(&username).await
}

#[tauri::command]
pub async fn remove_local_profile(state: State<'_, AppState>, user_id: u64) -> AppResult<PeopleOverview> {
    state.people.remove_profile(user_id).await
}

#[tauri::command]
pub async fn set_active_local_profile(state: State<'_, AppState>, user_id: u64) -> AppResult<PeopleOverview> {
    state.people.set_active(user_id).await
}

#[tauri::command]
pub async fn add_person(state: State<'_, AppState>, username: String) -> AppResult<PeopleOverview> {
    state.people.add_person(&username).await
}

#[tauri::command]
pub async fn remove_person(state: State<'_, AppState>, user_id: u64) -> AppResult<PeopleOverview> {
    state.people.remove_person(user_id).await
}

#[tauri::command]
pub fn clear_people(state: State<'_, AppState>) -> AppResult<()> {
    state.people.clear()
}

/// Built from a numeric id, so the frontend can't open an arbitrary address.
#[tauri::command]
pub fn open_user_profile(app: AppHandle, user_id: u64) -> AppResult<()> {
    if user_id == 0 {
        return Err(RobloxError::UserNotFound.into());
    }
    app.opener()
        .open_url(format!("https://www.roblox.com/users/{user_id}/profile"), None::<&str>)?;
    Ok(())
}
