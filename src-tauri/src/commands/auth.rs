use tauri::{AppHandle, State};

use crate::auth::AuthStatus;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn get_auth_status(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.status()
}

#[tauri::command]
pub fn set_oauth_client_id(state: State<'_, AppState>, client_id: String) -> AppResult<AuthStatus> {
    state.auth.set_client_id(&client_id)
}

#[tauri::command]
pub fn start_sign_in(app: AppHandle, state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.begin_sign_in(&app)
}

#[tauri::command]
pub fn cancel_sign_in(app: AppHandle, state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.cancel(&app)
}

#[tauri::command]
pub fn sign_out(app: AppHandle, state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.sign_out(&app)
}

#[tauri::command]
pub fn open_oauth_dashboard(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.auth.open_dashboard(&app)
}

#[tauri::command]
pub fn dismiss_welcome(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    state.auth.dismiss_welcome()
}
