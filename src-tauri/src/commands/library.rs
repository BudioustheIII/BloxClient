use tauri::State;

use crate::error::AppResult;
use crate::library::LibraryOverview;
use crate::roblox::LaunchTarget;
use crate::state::AppState;

#[tauri::command]
pub fn get_library(state: State<'_, AppState>) -> AppResult<LibraryOverview> {
    state.library.overview()
}

#[tauri::command]
pub fn set_game_favorite(state: State<'_, AppState>, target: LaunchTarget, favorite: bool) -> AppResult<LibraryOverview> {
    state.library.set_favorite(&target, favorite)
}

#[tauri::command]
pub fn set_game_pinned(state: State<'_, AppState>, target: LaunchTarget, pinned: bool) -> AppResult<LibraryOverview> {
    state.library.set_pinned(&target, pinned)
}

#[tauri::command]
pub fn create_collection(state: State<'_, AppState>, name: String) -> AppResult<LibraryOverview> {
    state.library.create_collection(&name)
}

#[tauri::command]
pub fn rename_collection(state: State<'_, AppState>, id: String, name: String) -> AppResult<LibraryOverview> {
    state.library.rename_collection(&id, &name)
}

#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> AppResult<LibraryOverview> {
    state.library.delete_collection(&id)
}

#[tauri::command]
pub fn set_game_in_collection(
    state: State<'_, AppState>,
    id: String,
    target: LaunchTarget,
    included: bool,
) -> AppResult<LibraryOverview> {
    state.library.set_in_collection(&id, &target, included)
}

#[tauri::command]
pub fn clear_library(state: State<'_, AppState>) -> AppResult<LibraryOverview> {
    state.library.clear()
}
