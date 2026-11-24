//! Starts experiences through the `roblox://` link and follows the Player's
//! process and window. The watcher thread exists only during a session.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use super::error::RobloxError;
use super::history;
use super::install::registered_launcher;
use super::model::{LaunchTarget, PlaySession, SessionStage};
use super::process::{empty_system, player_pids};
use crate::db::Database;
use crate::error::AppResult;

const SESSION_EVENT: &str = "play-session";
/// Launchers may update Roblox first, so this is deliberately generous.
const START_TIMEOUT: Duration = Duration::from_secs(150);
const START_POLL: Duration = Duration::from_secs(1);
const RUN_POLL: Duration = Duration::from_secs(5);
/// Roblox can briefly exit and restart during teleports.
const EXIT_GRACE: Duration = Duration::from_secs(3);
/// With Roblox already open, the running Player may take the game itself.
const HANDOFF_WAIT: Duration = Duration::from_secs(6);

pub fn launch_uri(place_id: u64) -> String {
    format!("roblox://experiences/start?placeId={place_id}")
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// How a launch resolved, given the Player processes seen so far.
#[derive(Debug, PartialEq, Eq)]
enum StartCheck {
    Started,
    Waiting,
    TimedOut,
}

fn check_start(
    baseline: &HashSet<u32>,
    current: &HashSet<u32>,
    elapsed: Duration,
) -> StartCheck {
    if current.difference(baseline).next().is_some() {
        return StartCheck::Started;
    }
    if !baseline.is_empty() && elapsed >= HANDOFF_WAIT && baseline.intersection(current).next().is_some() {
        return StartCheck::Started;
    }
    if elapsed >= START_TIMEOUT {
        StartCheck::TimedOut
    } else {
        StartCheck::Waiting
    }
}

pub struct SessionManager {
    db: Arc<Database>,
    current: Mutex<Option<PlaySession>>,
}

impl SessionManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            current: Mutex::new(None),
        }
    }

    pub fn current(&self) -> Option<PlaySession> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn launch(self: &Arc<Self>, app: &AppHandle, target: LaunchTarget, force: bool) -> AppResult<PlaySession> {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if current.is_some() {
            return Err(RobloxError::SessionActive.into());
        }

        let mut system = empty_system();
        let baseline = player_pids(&mut system);
        if !baseline.is_empty() && !force {
            return Err(RobloxError::AlreadyRunning.into());
        }

        open_link(app, target.place_id)?;

        let session = PlaySession {
            universe_id: target.universe_id,
            place_id: target.place_id,
            name: target.name.clone(),
            thumbnail_url: target.thumbnail_url.clone(),
            stage: SessionStage::Launching,
            running_since: None,
            message: None,
        };
        *current = Some(session.clone());
        drop(current);
        emit(app, &session);

        let manager = Arc::clone(self);
        let handle = app.clone();
        thread::spawn(move || manager.watch(&handle, target, baseline, system));
        Ok(session)
    }

    fn watch(&self, app: &AppHandle, target: LaunchTarget, baseline: HashSet<u32>, mut system: sysinfo::System) {
        let started = Instant::now();
        loop {
            thread::sleep(START_POLL);
            match check_start(&baseline, &player_pids(&mut system), started.elapsed()) {
                StartCheck::Started => break,
                StartCheck::Waiting => {}
                StartCheck::TimedOut => {
                    self.finish(
                        app,
                        SessionStage::Failed,
                        Some("Roblox didn't start. Check that it's installed and try again.".to_string()),
                    );
                    return;
                }
            }
        }

        let running_since = unix_now();
        if let Err(error) = history::record_play(&self.db, &target, running_since) {
            log::error!("could not save play history: {error}");
        }
        self.update(app, |session| {
            session.stage = SessionStage::Running;
            session.running_since = Some(running_since as f64);
        });
        log::info!("play session started");

        loop {
            thread::sleep(RUN_POLL);
            if player_pids(&mut system).is_empty() {
                thread::sleep(EXIT_GRACE);
                if player_pids(&mut system).is_empty() {
                    break;
                }
            }
        }

        if let Err(error) = history::add_play_time(&self.db, target.universe_id, unix_now() - running_since) {
            log::error!("could not save play time: {error}");
        }
        log::info!("play session ended");
        self.finish(app, SessionStage::Ended, None);
    }

    fn update(&self, app: &AppHandle, change: impl FnOnce(&mut PlaySession)) {
        let snapshot = {
            let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
            current.as_mut().map(|session| {
                change(session);
                session.clone()
            })
        };
        if let Some(session) = snapshot {
            emit(app, &session);
        }
    }

    fn finish(&self, app: &AppHandle, stage: SessionStage, message: Option<String>) {
        let ended = self.current.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(mut session) = ended {
            session.stage = stage;
            session.message = message;
            emit(app, &session);
        }
    }
}

fn emit(app: &AppHandle, session: &PlaySession) {
    if let Err(error) = app.emit(SESSION_EVENT, session) {
        log::warn!("could not send session update: {error}");
    }
}

fn open_link(app: &AppHandle, place_id: u64) -> AppResult<()> {
    let uri = launch_uri(place_id);

    // Development-only hook so automated tests never join a real game.
    if cfg!(debug_assertions) {
        if let Some(path) = std::env::var_os("BLOXCLIENT_TEST_LAUNCH_LOG") {
            std::fs::write(path, &uri)?;
            return Ok(());
        }
    }

    let launcher = registered_launcher().ok_or(RobloxError::NotInstalled)?;
    if !Path::new(&launcher).is_file() {
        return Err(RobloxError::LauncherMissing(launcher).into());
    }
    app.opener().open_url(uri, None::<&str>)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pids(values: &[u32]) -> HashSet<u32> {
        values.iter().copied().collect()
    }

    #[test]
    fn builds_a_numeric_launch_link() {
        assert_eq!(launch_uri(606849621), "roblox://experiences/start?placeId=606849621");
    }

    #[test]
    fn new_process_means_started() {
        assert_eq!(check_start(&pids(&[]), &pids(&[10]), Duration::ZERO), StartCheck::Started);
        assert_eq!(check_start(&pids(&[1]), &pids(&[1, 2]), Duration::ZERO), StartCheck::Started);
    }

    #[test]
    fn waits_then_times_out_without_a_process() {
        assert_eq!(check_start(&pids(&[]), &pids(&[]), Duration::from_secs(5)), StartCheck::Waiting);
        assert_eq!(check_start(&pids(&[]), &pids(&[]), START_TIMEOUT), StartCheck::TimedOut);
    }

    #[test]
    fn existing_player_is_adopted_only_after_the_handoff_wait() {
        let baseline = pids(&[1]);
        assert_eq!(check_start(&baseline, &pids(&[1]), Duration::from_secs(1)), StartCheck::Waiting);
        assert_eq!(check_start(&baseline, &pids(&[1]), HANDOFF_WAIT), StartCheck::Started);
    }

    #[test]
    fn player_closing_during_handoff_is_not_adopted() {
        assert_eq!(check_start(&pids(&[1]), &pids(&[]), HANDOFF_WAIT), StartCheck::Waiting);
    }
}
