//! Read-only detection of what opens `roblox://` links. BloxClient respects
//! that choice and never registers itself or changes the protocol.

use std::path::Path;

use winreg::enums::HKEY_CLASSES_ROOT;
use winreg::RegKey;

use super::model::RobloxInstall;
use super::process::is_player_running;

const COMMAND_KEY: &str = r"roblox\shell\open\command";
const OFFICIAL_LAUNCHERS: &[&str] = &["robloxplayerbeta", "robloxplayerlauncher", "robloxplayerinstaller"];

/// Extracts the program from a registry command like `"C:\a b\x.exe" %1`.
pub fn command_executable(command: &str) -> Option<String> {
    let command = command.trim();
    let path = match command.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => command.split_whitespace().next()?,
    };
    (!path.is_empty()).then(|| path.to_string())
}

fn launcher_name(path: &str) -> (String, bool) {
    let stem = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    if OFFICIAL_LAUNCHERS.contains(&stem.to_lowercase().as_str()) {
        return ("Roblox".to_string(), true);
    }
    let mut letters = stem.chars();
    let name = match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => stem,
    };
    (name, false)
}

pub fn registered_launcher() -> Option<String> {
    let command: String = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(COMMAND_KEY)
        .ok()?
        .get_value("")
        .ok()?;
    command_executable(&command)
}

pub fn detect() -> RobloxInstall {
    let launcher_path = registered_launcher();
    let launcher_found = launcher_path.as_deref().is_some_and(|path| Path::new(path).is_file());
    let (launcher_name, is_official_launcher) = match launcher_path.as_deref().map(launcher_name) {
        Some((name, official)) => (Some(name), official),
        None => (None, false),
    };
    RobloxInstall {
        launcher_name,
        launcher_path,
        launcher_found,
        is_official_launcher,
        player_running: is_player_running(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_commands() {
        assert_eq!(
            command_executable(r#""C:\Program Files\Roblox\RobloxPlayerBeta.exe" %1"#).as_deref(),
            Some(r"C:\Program Files\Roblox\RobloxPlayerBeta.exe")
        );
        assert_eq!(
            command_executable(r#""C:\Tools\froststrap.exe" --player "%1""#).as_deref(),
            Some(r"C:\Tools\froststrap.exe")
        );
        assert_eq!(command_executable(r"C:\Tools\launcher.exe %1").as_deref(), Some(r"C:\Tools\launcher.exe"));
        assert_eq!(command_executable(""), None);
        assert_eq!(command_executable(r#""""#), None);
    }

    #[test]
    fn identifies_official_and_third_party_launchers() {
        assert_eq!(launcher_name(r"C:\R\RobloxPlayerBeta.exe"), ("Roblox".to_string(), true));
        assert_eq!(launcher_name(r"C:\F\froststrap.exe"), ("Froststrap".to_string(), false));
    }
}
