use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppResult;
use crate::state::AppState;
use crate::window;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub version: String,
    pub schema_version: i32,
    pub webview_version: Option<String>,
    pub platform: String,
    pub data_dir: String,
    pub log_dir: String,
    pub database_bytes: f64,
}

/// A fixed set of folders, so the frontend can never open an arbitrary path.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum AppFolder {
    Data,
    Logs,
}

#[tauri::command]
pub fn app_ready(app: AppHandle) {
    window::reveal_on_launch(&app);
}

#[tauri::command]
pub fn get_background_animation(app: AppHandle) -> bool {
    window::animation_allowed(&app)
}

#[tauri::command]
pub fn get_diagnostics(app: AppHandle, state: State<'_, AppState>) -> AppResult<Diagnostics> {
    let database_bytes = std::fs::metadata(&state.paths.database)?.len() as f64;
    Ok(Diagnostics {
        version: app.package_info().version.to_string(),
        schema_version: state.db.schema_version()? as i32,
        webview_version: tauri::webview_version().ok(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
        database_bytes,
    })
}

#[tauri::command]
pub fn open_app_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder: AppFolder,
) -> AppResult<()> {
    let path = match folder {
        AppFolder::Data => &state.paths.data_dir,
        AppFolder::Logs => &state.paths.log_dir,
    };
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)?;
    Ok(())
}
