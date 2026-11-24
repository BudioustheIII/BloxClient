use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::fflags::{FlagInput, FlagOverview};
use crate::state::AppState;

const MAX_NAME_ATTEMPTS: u32 = 100;

#[tauri::command]
pub fn get_fflags(state: State<'_, AppState>) -> AppResult<FlagOverview> {
    state.fflags.overview()
}

#[tauri::command]
pub fn set_active_fflag_profile(state: State<'_, AppState>, id: String) -> AppResult<FlagOverview> {
    state.fflags.set_active(&id)
}

#[tauri::command]
pub fn create_fflag_profile(
    state: State<'_, AppState>,
    name: String,
    copy_from: Option<String>,
) -> AppResult<FlagOverview> {
    state.fflags.create_profile(&name, copy_from.as_deref())
}

#[tauri::command]
pub fn rename_fflag_profile(state: State<'_, AppState>, id: String, name: String) -> AppResult<FlagOverview> {
    state.fflags.rename_profile(&id, &name)
}

#[tauri::command]
pub fn delete_fflag_profile(state: State<'_, AppState>, id: String) -> AppResult<FlagOverview> {
    state.fflags.delete_profile(&id)
}

#[tauri::command]
pub fn save_fflag(
    state: State<'_, AppState>,
    profile_id: String,
    original: Option<String>,
    flag: FlagInput,
) -> AppResult<FlagOverview> {
    state.fflags.save_flag(&profile_id, original.as_deref(), flag)
}

#[tauri::command]
pub fn set_fflag_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    name: String,
    enabled: bool,
) -> AppResult<FlagOverview> {
    state.fflags.set_flag_enabled(&profile_id, &name, enabled)
}

#[tauri::command]
pub fn remove_fflag(state: State<'_, AppState>, profile_id: String, name: String) -> AppResult<FlagOverview> {
    state.fflags.remove_flag(&profile_id, &name)
}

#[tauri::command]
pub fn import_fflags(
    state: State<'_, AppState>,
    profile_id: String,
    text: String,
    replace: bool,
) -> AppResult<FlagOverview> {
    state.fflags.import(&profile_id, &text, replace)
}

#[tauri::command]
pub fn get_fflags_json(state: State<'_, AppState>, profile_id: String) -> AppResult<String> {
    state.fflags.export_json(&profile_id)
}

#[tauri::command]
pub fn reset_fflags(state: State<'_, AppState>) -> AppResult<FlagOverview> {
    state.fflags.reset()
}

#[tauri::command]
pub fn restore_fflag_backup(state: State<'_, AppState>, id: i64) -> AppResult<FlagOverview> {
    state.fflags.restore_backup(id)
}

fn file_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|item| if item.is_ascii_alphanumeric() { item.to_ascii_lowercase() } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() { "profile".to_string() } else { slug }
}

/// Saves to Downloads under a new name, so an existing file is never replaced.
#[tauri::command]
pub fn export_fflags_file(app: AppHandle, state: State<'_, AppState>, profile_id: String) -> AppResult<String> {
    let contents = state.fflags.export_json(&profile_id)?;
    let slug = file_slug(&state.fflags.profile_name(&profile_id)?);
    let folder: PathBuf = app.path().download_dir()?;

    for attempt in 0..MAX_NAME_ATTEMPTS {
        let suffix = if attempt == 0 { String::new() } else { format!("-{}", attempt + 1) };
        let path = folder.join(format!("BloxClient-flags-{slug}{suffix}.json"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(contents.as_bytes())?;
                if let Err(error) = app.opener().reveal_item_in_dir(&path) {
                    log::warn!("could not show exported flags: {error}");
                }
                return Ok(path.display().to_string());
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(AppError::Io(std::io::Error::new(ErrorKind::AlreadyExists, "no free export file name")))
}

#[cfg(test)]
mod tests {
    use super::file_slug;

    #[test]
    fn export_names_are_safe() {
        assert_eq!(file_slug("My Quality!"), "my-quality");
        assert_eq!(file_slug("..\\..\\evil"), "evil");
        assert_eq!(file_slug("日本語"), "profile");
    }
}
