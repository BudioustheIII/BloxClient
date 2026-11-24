//! The "launch on Windows startup" entry, stored per user under the Run key.

use std::io::{self, ErrorKind};

use serde_json::json;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

use crate::error::{AppError, AppResult};
use crate::settings::SettingsStore;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "BloxClient";

fn launch_command() -> io::Result<String> {
    Ok(format!("\"{}\"", std::env::current_exe()?.display()))
}

pub fn set_enabled(enabled: bool) -> AppResult<()> {
    let result = if enabled {
        launch_command().and_then(|command| write_entry(RUN_KEY, VALUE_NAME, &command))
    } else {
        remove_entry(RUN_KEY, VALUE_NAME)
    };
    result.map_err(AppError::Autostart)
}

#[derive(Debug, PartialEq, Eq)]
enum Repair {
    Nothing,
    WriteEntry,
    TurnOff,
}

/// A missing entry normally means the user turned startup off outside the
/// app. Right after a version change it means the installer's default upgrade
/// ran the full uninstaller, so the user's preference is restored instead.
fn repair_for(registered: Option<&str>, expected: &str, version_changed: bool) -> Repair {
    match registered {
        Some(command) if command == expected => Repair::Nothing,
        Some(_) => Repair::WriteEntry,
        None if version_changed => Repair::WriteEntry,
        None => Repair::TurnOff,
    }
}

/// Aligns the saved preference with Windows.
pub fn reconcile(settings: &SettingsStore, version: &str) -> AppResult<()> {
    let version_changed = settings.last_run_version()?.as_deref() != Some(version);

    if settings.get().general.launch_on_startup {
        let registered = read_entry(RUN_KEY, VALUE_NAME).map_err(AppError::Autostart)?;
        let expected = launch_command().map_err(AppError::Autostart)?;
        match repair_for(registered.as_deref(), &expected, version_changed) {
            Repair::Nothing => {}
            Repair::WriteEntry => {
                write_entry(RUN_KEY, VALUE_NAME, &expected).map_err(AppError::Autostart)?;
            }
            Repair::TurnOff => {
                settings.update(&json!({ "general": { "launchOnStartup": false } }), |_, _| {
                    Ok(())
                })?;
            }
        }
    }

    if version_changed {
        settings.set_last_run_version(version)?;
    }
    Ok(())
}

fn read_entry(subkey: &str, name: &str) -> io::Result<Option<String>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(subkey, KEY_READ) {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    match key.get_value(name) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_entry(subkey: &str, name: &str, command: &str) -> io::Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(subkey)?;
    key.set_value(name, &command)
}

fn remove_entry(subkey: &str, name: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(subkey, KEY_SET_VALUE) {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    match key.delete_value(name) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Uses a private key so tests never touch the real Windows startup list.
    const TEST_KEY: &str = r"Software\BloxClientTests\Run";

    #[test]
    fn upgrade_restores_an_entry_the_installer_removed() {
        let expected = r#""C:\Apps\BloxClient.exe""#;
        assert_eq!(repair_for(None, expected, true), Repair::WriteEntry);
    }

    #[test]
    fn removed_entry_without_an_upgrade_turns_startup_off() {
        let expected = r#""C:\Apps\BloxClient.exe""#;
        assert_eq!(repair_for(None, expected, false), Repair::TurnOff);
    }

    #[test]
    fn existing_entries_are_kept_or_corrected() {
        let expected = r#""C:\Apps\BloxClient.exe""#;
        assert_eq!(repair_for(Some(expected), expected, true), Repair::Nothing);
        assert_eq!(repair_for(Some(expected), expected, false), Repair::Nothing);
        assert_eq!(repair_for(Some(r#""C:\Old.exe""#), expected, false), Repair::WriteEntry);
    }

    #[test]
    fn entry_round_trip() {
        let name = "round_trip";
        remove_entry(TEST_KEY, name).unwrap();
        assert_eq!(read_entry(TEST_KEY, name).unwrap(), None);

        write_entry(TEST_KEY, name, r#""C:\Apps\BloxClient.exe""#).unwrap();
        assert_eq!(
            read_entry(TEST_KEY, name).unwrap().as_deref(),
            Some(r#""C:\Apps\BloxClient.exe""#)
        );

        remove_entry(TEST_KEY, name).unwrap();
        remove_entry(TEST_KEY, name).unwrap();
        assert_eq!(read_entry(TEST_KEY, name).unwrap(), None);

        RegKey::predef(HKEY_CURRENT_USER)
            .delete_subkey_all(r"Software\BloxClientTests")
            .unwrap();
    }
}
