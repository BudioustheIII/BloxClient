use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::autostart;
use crate::error::AppResult;
use crate::settings::Settings;
use crate::state::AppState;
use crate::tray;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: Settings,
    pub version: String,
}

#[tauri::command]
pub fn get_bootstrap(app: AppHandle, state: State<'_, AppState>) -> Bootstrap {
    Bootstrap {
        settings: state.settings.get(),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: Value,
) -> AppResult<Settings> {
    state
        .settings
        .update(&patch, |previous, next| apply_native(&app, previous, next))
}

#[tauri::command]
pub fn reset_settings(app: AppHandle, state: State<'_, AppState>) -> AppResult<Settings> {
    state
        .settings
        .reset(|previous, next| apply_native(&app, previous, next))
}

/// Applies the settings that live outside the database.
fn apply_native(app: &AppHandle, previous: &Settings, next: &Settings) -> AppResult<()> {
    let (before, after) = (&previous.general, &next.general);

    if before.minimize_to_tray != after.minimize_to_tray {
        tray::sync(app, after.minimize_to_tray)?;
    }
    if before.launch_on_startup != after.launch_on_startup {
        if let Err(error) = autostart::set_enabled(after.launch_on_startup) {
            if let Err(undo) = tray::sync(app, before.minimize_to_tray) {
                log::error!("could not restore tray state: {undo}");
            }
            return Err(error);
        }
    }
    Ok(())
}
