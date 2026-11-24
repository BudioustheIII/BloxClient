use tauri::State;

use crate::error::AppResult;
use crate::performance::{NetworkProbe, PerfPoint};
use crate::state::AppState;

#[tauri::command]
pub fn get_performance_history(state: State<'_, AppState>) -> Vec<PerfPoint> {
    state.performance.history()
}

/// Counter reads can block briefly, so they stay off the main thread.
#[tauri::command]
pub async fn sample_performance(state: State<'_, AppState>) -> AppResult<Option<PerfPoint>> {
    let monitor = state.performance.clone();
    Ok(tauri::async_runtime::spawn_blocking(move || monitor.sample()).await?)
}

#[tauri::command]
pub async fn check_network(state: State<'_, AppState>) -> AppResult<Vec<NetworkProbe>> {
    Ok(state.performance.check_network().await)
}
