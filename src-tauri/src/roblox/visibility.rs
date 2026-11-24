//! Hides BloxClient while Roblox is open and brings it back afterward,
//! however Roblox was started. Roblox counts as open while its window exists.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use super::process::{empty_system, player_pids};
use crate::state::AppState;
use crate::window;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Roblox can briefly exit and restart during teleports.
const MISSED_POLLS_TO_CLOSE: u8 = 2;
pub const PLAY_HISTORY_EVENT: &str = "play-history-changed";

#[derive(Debug, PartialEq, Eq)]
enum Transition {
    None,
    Opened,
    Closed,
}

struct Tracker {
    open: bool,
    missed: u8,
}

impl Tracker {
    /// Roblox already open at startup is not a launch, so it never hides the
    /// window the user just opened.
    fn new(open: bool) -> Self {
        Self { open, missed: 0 }
    }

    fn observe(&mut self, running: bool) -> Transition {
        if running {
            self.missed = 0;
            if self.open {
                return Transition::None;
            }
            self.open = true;
            return Transition::Opened;
        }
        if !self.open {
            return Transition::None;
        }
        self.missed += 1;
        if self.missed < MISSED_POLLS_TO_CLOSE {
            return Transition::None;
        }
        self.open = false;
        self.missed = 0;
        Transition::Closed
    }
}

pub fn spawn_player_watcher(app: AppHandle) {
    thread::spawn(move || {
        let mut system = empty_system();
        let open_at_start = !player_pids(&mut system).is_empty();
        let mut tracker = Tracker::new(open_at_start);
        let mut hidden_by_us = false;
        window::set_player_open(&app, open_at_start);

        loop {
            thread::sleep(POLL_INTERVAL);
            match tracker.observe(!player_pids(&mut system).is_empty()) {
                Transition::None => {}
                Transition::Opened => {
                    window::set_player_open(&app, true);
                    hidden_by_us = window::hide_for_player(&app);
                }
                Transition::Closed => {
                    window::set_player_open(&app, false);
                    if hidden_by_us {
                        window::restore_after_player(&app);
                    }
                    hidden_by_us = false;
                    sync_history(&app);
                }
            }
        }
    });
}

fn sync_history(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        match state.play_history.sync(&state.catalog).await {
            Ok(true) => {
                if let Err(error) = handle.emit(PLAY_HISTORY_EVENT, ()) {
                    log::warn!("could not send history update: {error}");
                }
            }
            Ok(false) => {}
            Err(error) => log::warn!("could not update play history: {error}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_open_once() {
        let mut tracker = Tracker::new(false);
        assert_eq!(tracker.observe(false), Transition::None);
        assert_eq!(tracker.observe(true), Transition::Opened);
        assert_eq!(tracker.observe(true), Transition::None);
    }

    #[test]
    fn close_needs_consecutive_missed_polls() {
        let mut tracker = Tracker::new(false);
        tracker.observe(true);
        assert_eq!(tracker.observe(false), Transition::None);
        assert_eq!(tracker.observe(true), Transition::None, "a teleport restart isn't a close");
        assert_eq!(tracker.observe(false), Transition::None);
        assert_eq!(tracker.observe(false), Transition::Closed);
        assert_eq!(tracker.observe(false), Transition::None);
    }

    #[test]
    fn roblox_open_at_startup_is_not_a_launch() {
        let mut tracker = Tracker::new(true);
        assert_eq!(tracker.observe(true), Transition::None);
    }

    #[test]
    fn can_open_again_after_closing() {
        let mut tracker = Tracker::new(false);
        tracker.observe(true);
        tracker.observe(false);
        tracker.observe(false);
        assert_eq!(tracker.observe(true), Transition::Opened);
    }
}
