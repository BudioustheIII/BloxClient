//! Local play history, encrypted at rest. It never leaves this PC.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::model::{LaunchTarget, RecentExperience};
use crate::db::Database;
use crate::error::AppResult;
use crate::vault;

const HISTORY_KEY: &str = "play_history";
const LEGACY_CLEARED_KEY: &str = "history.cleared_at";
const MAX_ENTRIES: usize = 500;
const MAX_NAME_CHARS: usize = 200;
const TRUSTED_IMAGE_PREFIX: &str = "https://tr.rbxcdn.com/";

/// A game found in Roblox's logs, with details if they could be loaded.
pub struct ImportedPlay {
    pub universe_id: u64,
    pub played_at: i64,
    pub game: Option<LaunchTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    universe_id: u64,
    place_id: u64,
    name: String,
    creator_name: Option<String>,
    thumbnail_url: Option<String>,
    last_played_at: i64,
    play_count: i64,
    total_seconds: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryDoc {
    cleared_at: i64,
    entries: Vec<Entry>,
}

impl HistoryDoc {
    fn sort_and_trim(&mut self) {
        self.entries
            .sort_by(|a, b| b.last_played_at.cmp(&a.last_played_at).then(a.universe_id.cmp(&b.universe_id)));
        self.entries.truncate(MAX_ENTRIES);
    }

    fn find(&mut self, universe_id: u64) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| entry.universe_id == universe_id)
    }
}

pub fn bounded(text: &str) -> String {
    text.chars().take(MAX_NAME_CHARS).collect()
}

pub fn trusted_image(url: Option<&str>) -> Option<String> {
    url.filter(|url| url.starts_with(TRUSTED_IMAGE_PREFIX)).map(str::to_string)
}

fn read(db: &Database) -> AppResult<HistoryDoc> {
    Ok(vault::read(&db.connection(), HISTORY_KEY)?.unwrap_or_default())
}

/// Holds the database lock across the whole read, change and write.
fn update<R>(db: &Database, change: impl FnOnce(&mut HistoryDoc) -> R) -> AppResult<R> {
    let connection = db.connection();
    let mut doc: HistoryDoc = vault::read(&connection, HISTORY_KEY)?.unwrap_or_default();
    let result = change(&mut doc);
    doc.sort_and_trim();
    vault::write(&connection, HISTORY_KEY, &doc)?;
    Ok(result)
}

fn apply_details(entry: &mut Entry, game: &LaunchTarget) {
    entry.place_id = game.place_id;
    entry.name = bounded(&game.name);
    if let Some(creator) = &game.creator_name {
        entry.creator_name = Some(bounded(creator));
    }
    if let Some(url) = trusted_image(game.thumbnail_url.as_deref()) {
        entry.thumbnail_url = Some(url);
    }
}

fn new_entry(game: &LaunchTarget, played_at: i64) -> Entry {
    Entry {
        universe_id: game.universe_id,
        place_id: game.place_id,
        name: bounded(&game.name),
        creator_name: game.creator_name.as_deref().map(bounded),
        thumbnail_url: trusted_image(game.thumbnail_url.as_deref()),
        last_played_at: played_at,
        play_count: 1,
        total_seconds: 0,
    }
}

pub fn record_play(db: &Database, target: &LaunchTarget, now: i64) -> AppResult<()> {
    update(db, |doc| match doc.find(target.universe_id) {
        Some(entry) => {
            apply_details(entry, target);
            entry.last_played_at = now;
            entry.play_count += 1;
        }
        None => doc.entries.push(new_entry(target, now)),
    })
}

pub fn add_play_time(db: &Database, universe_id: u64, seconds: i64) -> AppResult<()> {
    update(db, |doc| {
        if let Some(entry) = doc.find(universe_id) {
            entry.total_seconds += seconds.max(0);
        }
    })
}

pub fn list(db: &Database, limit: u32) -> AppResult<Vec<RecentExperience>> {
    let mut doc = read(db)?;
    doc.sort_and_trim();
    Ok(doc
        .entries
        .into_iter()
        .take(limit as usize)
        .map(|entry| RecentExperience {
            universe_id: entry.universe_id,
            place_id: entry.place_id,
            name: entry.name,
            creator_name: entry.creator_name,
            thumbnail_url: entry.thumbnail_url,
            last_played_at: entry.last_played_at as f64,
            play_count: entry.play_count as f64,
            total_seconds: entry.total_seconds as f64,
        })
        .collect())
}

/// Clears history and remembers when, so the same plays aren't re-imported
/// from Roblox's logs.
pub fn clear(db: &Database) -> AppResult<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    update(db, |doc| {
        doc.entries.clear();
        doc.cleared_at = now;
    })
}

pub fn cleared_at(db: &Database) -> AppResult<i64> {
    Ok(read(db)?.cleared_at)
}

pub fn last_played_times(db: &Database) -> AppResult<HashMap<u64, i64>> {
    Ok(read(db)?
        .entries
        .into_iter()
        .map(|entry| (entry.universe_id, entry.last_played_at))
        .collect())
}

/// Saves plays found in Roblox's logs. A game without details can only update
/// an existing entry. Play counts aren't changed, since logs can be re-read.
pub fn apply_imports(db: &Database, imports: &[ImportedPlay]) -> AppResult<usize> {
    update(db, |doc| {
        let mut changed = 0;
        for import in imports {
            match (doc.find(import.universe_id), &import.game) {
                (Some(entry), game) => {
                    if let Some(game) = game {
                        apply_details(entry, game);
                    }
                    entry.last_played_at = entry.last_played_at.max(import.played_at);
                    changed += 1;
                }
                (None, Some(game)) => {
                    doc.entries.push(new_entry(game, import.played_at));
                    changed += 1;
                }
                (None, None) => {}
            }
        }
        changed
    })
}

/// Moves history from the old unencrypted table into the vault, then removes
/// the table and rewrites the database file so the old text is gone.
pub fn migrate_plaintext(db: &Database) -> AppResult<()> {
    let connection = db.connection();
    let exists: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'recent_experiences')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(());
    }

    let legacy: Vec<Entry> = {
        let mut statement = connection.prepare(
            "SELECT universe_id, place_id, name, creator_name, thumbnail_url, last_played_at, play_count, total_seconds
             FROM recent_experiences",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Entry {
                universe_id: row.get::<_, i64>(0)? as u64,
                place_id: row.get::<_, i64>(1)? as u64,
                name: row.get(2)?,
                creator_name: row.get(3)?,
                thumbnail_url: row.get(4)?,
                last_played_at: row.get(5)?,
                play_count: row.get(6)?,
                total_seconds: row.get(7)?,
            })
        })?;
        rows.collect::<Result<_, _>>()?
    };
    let legacy_cleared: Option<String> = connection
        .query_row("SELECT value FROM settings WHERE key = ?1", [LEGACY_CLEARED_KEY], |row| row.get(0))
        .optional()?;

    let mut doc: HistoryDoc = vault::read(&connection, HISTORY_KEY)?.unwrap_or_default();
    doc.cleared_at = doc
        .cleared_at
        .max(legacy_cleared.and_then(|value| value.parse().ok()).unwrap_or(0));
    for entry in legacy {
        if !doc.entries.iter().any(|existing| existing.universe_id == entry.universe_id) {
            doc.entries.push(entry);
        }
    }
    doc.sort_and_trim();
    vault::write(&connection, HISTORY_KEY, &doc)?;

    connection.execute("DELETE FROM settings WHERE key = ?1", [LEGACY_CLEARED_KEY])?;
    connection.execute_batch("DROP TABLE recent_experiences; VACUUM;")?;
    // In-memory databases have no write-ahead log to truncate.
    let _ = connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: u64, name: &str) -> LaunchTarget {
        LaunchTarget {
            universe_id: id,
            place_id: id * 10,
            name: name.to_string(),
            creator_name: Some("Maker".to_string()),
            thumbnail_url: Some("https://tr.rbxcdn.com/image".to_string()),
        }
    }

    #[test]
    fn replays_update_one_row_and_sort_newest_first() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, "One"), 100).unwrap();
        record_play(&db, &target(2, "Two"), 200).unwrap();
        record_play(&db, &target(1, "One renamed"), 300).unwrap();
        add_play_time(&db, 1, 60).unwrap();
        add_play_time(&db, 1, 30).unwrap();

        let recent = list(&db, 10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].universe_id, 1);
        assert_eq!(recent[0].name, "One renamed");
        assert_eq!(recent[0].play_count, 2.0);
        assert_eq!(recent[0].total_seconds, 90.0);
    }

    #[test]
    fn untrusted_image_urls_are_not_stored() {
        let db = Database::in_memory().unwrap();
        let mut bad = target(1, "One");
        bad.thumbnail_url = Some("https://evil.example/x.png".to_string());
        record_play(&db, &bad, 100).unwrap();
        assert_eq!(list(&db, 1).unwrap()[0].thumbnail_url, None);
    }

    #[test]
    fn missing_metadata_keeps_previous_values() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, "One"), 100).unwrap();
        let mut sparse = target(1, "One");
        sparse.creator_name = None;
        sparse.thumbnail_url = None;
        record_play(&db, &sparse, 200).unwrap();
        let recent = &list(&db, 1).unwrap()[0];
        assert_eq!(recent.creator_name.as_deref(), Some("Maker"));
        assert!(recent.thumbnail_url.is_some());
    }

    #[test]
    fn imports_add_games_without_inflating_play_counts() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, "One"), 100).unwrap();
        let imports = [
            ImportedPlay { universe_id: 1, played_at: 500, game: Some(target(1, "One")) },
            ImportedPlay { universe_id: 2, played_at: 400, game: Some(target(2, "Two")) },
        ];
        assert_eq!(apply_imports(&db, &imports).unwrap(), 2);
        apply_imports(&db, &imports).unwrap();

        let recent = list(&db, 10).unwrap();
        assert_eq!(recent.iter().map(|item| item.universe_id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(recent[0].play_count, 1.0);
        assert_eq!(recent[0].last_played_at, 500.0);
    }

    #[test]
    fn imports_never_move_a_game_back_in_time() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, "One"), 900).unwrap();
        apply_imports(&db, &[ImportedPlay { universe_id: 1, played_at: 100, game: Some(target(1, "One")) }]).unwrap();
        assert_eq!(list(&db, 1).unwrap()[0].last_played_at, 900.0);
    }

    #[test]
    fn imports_without_details_only_update_known_games() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, "One"), 100).unwrap();
        let imports = [
            ImportedPlay { universe_id: 1, played_at: 200, game: None },
            ImportedPlay { universe_id: 2, played_at: 300, game: None },
        ];
        assert_eq!(apply_imports(&db, &imports).unwrap(), 1);
        let recent = list(&db, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].last_played_at, 200.0);
    }

    #[test]
    fn clear_records_when_it_happened_and_keeps_settings() {
        let db = Database::in_memory().unwrap();
        assert_eq!(cleared_at(&db).unwrap(), 0);
        db.connection()
            .execute("INSERT INTO settings (key, value) VALUES ('app', '{}')", [])
            .unwrap();
        record_play(&db, &target(1, "One"), 100).unwrap();
        clear(&db).unwrap();
        assert!(cleared_at(&db).unwrap() > 0);
        assert!(list(&db, 10).unwrap().is_empty());
        let kept: i64 = db
            .connection()
            .query_row("SELECT count(*) FROM settings WHERE key = 'app'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kept, 1);
    }

    #[test]
    fn long_names_are_bounded() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, &"x".repeat(5000)), 100).unwrap();
        assert_eq!(list(&db, 1).unwrap()[0].name.len(), MAX_NAME_CHARS);
    }

    #[test]
    fn history_is_not_stored_in_plain_text() {
        let db = Database::in_memory().unwrap();
        record_play(&db, &target(1, "Very Private Game"), 100).unwrap();
        let raw: Vec<u8> = db
            .connection()
            .query_row("SELECT value FROM vault WHERE key = ?1", [HISTORY_KEY], |row| row.get(0))
            .unwrap();
        assert!(!raw.windows(7).any(|window| window == b"Private"));
    }

    #[test]
    fn old_plain_history_moves_into_the_vault() {
        let db = Database::in_memory().unwrap();
        {
            let connection = db.connection();
            connection
                .execute(
                    "INSERT INTO recent_experiences
                         (universe_id, place_id, name, creator_name, thumbnail_url, last_played_at, play_count, total_seconds)
                     VALUES (7, 70, 'Old Game', 'Maker', NULL, 500, 3, 120)",
                    [],
                )
                .unwrap();
            connection
                .execute("INSERT INTO settings (key, value) VALUES (?1, '400')", [LEGACY_CLEARED_KEY])
                .unwrap();
        }

        migrate_plaintext(&db).unwrap();
        migrate_plaintext(&db).unwrap();

        let recent = list(&db, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "Old Game");
        assert_eq!(recent[0].play_count, 3.0);
        assert_eq!(cleared_at(&db).unwrap(), 400);
        let tables: i64 = db
            .connection()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'recent_experiences'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }
}
