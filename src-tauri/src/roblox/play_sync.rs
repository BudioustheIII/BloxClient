//! Merges games from the Roblox logs into BloxClient's saved play history.
//! Roblox deletes old logs, so saved history outlives them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::catalog::Catalog;
use super::history::{self, ImportedPlay};
use super::player_logs::{read_player_logs, LogPlay};
use crate::db::Database;
use crate::error::AppResult;

pub struct PlayHistorySync {
    db: Arc<Database>,
    logs: PathBuf,
    running: AtomicBool,
}

struct RunGuard<'a>(&'a AtomicBool);

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// The newest play per game that isn't saved yet and wasn't cleared by the user.
fn new_plays(plays: Vec<LogPlay>, saved: &HashMap<u64, i64>, cleared_at: i64) -> HashMap<u64, i64> {
    let mut latest: HashMap<u64, i64> = HashMap::new();
    for play in plays {
        let entry = latest.entry(play.universe_id).or_insert(play.played_at);
        *entry = (*entry).max(play.played_at);
    }
    latest.retain(|universe, played_at| {
        *played_at > cleared_at && saved.get(universe).is_none_or(|last| *played_at > *last)
    });
    latest
}

impl PlayHistorySync {
    pub fn new(db: Arc<Database>, logs: PathBuf) -> Self {
        Self {
            db,
            logs,
            running: AtomicBool::new(false),
        }
    }

    /// Returns true if history changed. Contacts Roblox only for new plays,
    /// to get their names and images.
    pub async fn sync(&self, catalog: &Catalog) -> AppResult<bool> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(false);
        }
        let _guard = RunGuard(&self.running);

        let folder = self.logs.clone();
        let plays = tauri::async_runtime::spawn_blocking(move || read_player_logs(&folder)).await?;
        let changed = new_plays(plays, &history::last_played_times(&self.db)?, history::cleared_at(&self.db)?);
        if changed.is_empty() {
            return Ok(false);
        }

        let ids: Vec<u64> = changed.keys().copied().collect();
        let mut games = catalog.launch_targets(&ids).await;
        let imports: Vec<ImportedPlay> = changed
            .into_iter()
            .map(|(universe_id, played_at)| ImportedPlay {
                universe_id,
                played_at,
                game: games.remove(&universe_id),
            })
            .collect();
        Ok(history::apply_imports(&self.db, &imports)? > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(universe_id: u64, played_at: i64) -> LogPlay {
        LogPlay { universe_id, played_at }
    }

    #[test]
    fn keeps_the_newest_play_per_game() {
        let result = new_plays(vec![play(1, 100), play(1, 300), play(1, 200), play(2, 50)], &HashMap::new(), 0);
        assert_eq!(result.get(&1), Some(&300));
        assert_eq!(result.get(&2), Some(&50));
    }

    #[test]
    fn skips_plays_already_saved() {
        let saved = HashMap::from([(1, 300), (2, 100)]);
        let result = new_plays(vec![play(1, 300), play(2, 200)], &saved, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&2), Some(&200));
    }

    #[test]
    fn cleared_history_is_not_reimported() {
        let result = new_plays(vec![play(1, 100), play(2, 500)], &HashMap::new(), 100);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key(&2));
    }
}
