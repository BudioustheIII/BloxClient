//! Detects an open Roblox Player from the process list and the desktop's
//! window list. BloxClient never opens or reads the Roblox process itself.

use std::collections::HashSet;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};

const PLAYER_PROCESS: &str = "RobloxPlayerBeta.exe";

/// Development runs can watch a stand-in, so automated tests never react to a
/// real Roblox session. Ignored in release builds.
fn watched_process() -> String {
    if cfg!(debug_assertions) {
        if let Ok(name) = std::env::var("BLOXCLIENT_TEST_PLAYER_PROCESS") {
            return name;
        }
    }
    PLAYER_PROCESS.to_string()
}

unsafe extern "system" fn collect_window_owner(window: HWND, owners: LPARAM) -> BOOL {
    // SAFETY: `owners` is the live HashSet passed by `processes_with_windows`,
    // and EnumWindows calls back synchronously on this thread.
    let owners = unsafe { &mut *(owners as *mut HashSet<u32>) };
    // SAFETY: `window` comes from EnumWindows, and these calls tolerate a
    // window that has since closed.
    unsafe {
        if IsWindowVisible(window) != 0 {
            let mut process_id = 0;
            GetWindowThreadProcessId(window, &mut process_id);
            owners.insert(process_id);
        }
    }
    1
}

fn processes_with_windows() -> HashSet<u32> {
    let mut owners = HashSet::new();
    // SAFETY: the callback only uses `owners`, which outlives this call.
    unsafe {
        EnumWindows(Some(collect_window_owner), (&raw mut owners) as LPARAM);
    }
    owners
}

pub fn empty_system() -> System {
    System::new_with_specifics(RefreshKind::nothing())
}

/// Player processes with a visible window. Roblox can leave a windowless
/// process behind after a game closes, and that doesn't count as open.
pub fn player_pids(system: &mut System) -> HashSet<u32> {
    let name = watched_process();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut pids: HashSet<u32> = system
        .processes()
        .iter()
        .filter(|(_, process)| process.name().eq_ignore_ascii_case(&name))
        .map(|(pid, _)| pid.as_u32())
        .collect();
    if !pids.is_empty() {
        let with_windows = processes_with_windows();
        pids.retain(|pid| with_windows.contains(pid));
    }
    pids
}

pub fn is_player_running() -> bool {
    !player_pids(&mut empty_system()).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_enumeration_finds_desktop_windows() {
        // Any interactive Windows session has at least one visible window.
        let owners = processes_with_windows();
        assert!(!owners.is_empty());
        assert!(!owners.contains(&u32::MAX));
    }
}
