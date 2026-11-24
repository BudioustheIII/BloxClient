//! FFlag profiles, backups and writing `ClientAppSettings.json`.
//!
//! Flags are only written into versions BloxClient installed itself, right
//! before launching them. Roblox's own folders and files owned by third-party
//! launchers are never changed.

mod error;
mod model;
mod presets;
mod validate;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Map;

pub use error::FlagError;
pub use model::{BackupInfo, FlagEntry, FlagInput, FlagOverview, FlagView, ProfileSource, ProfileView};

#[cfg(test)]
use model::FlagKind;

use crate::db::Database;
use crate::error::AppResult;
use presets::{DEFAULT_ID, PRESETS};
use validate::{check_name, normalize_value, parse_import, to_json, view, MAX_FLAGS};

const CUSTOM_ID: &str = "custom";
const MAX_PROFILES: usize = 50;
const MAX_PROFILE_NAME: usize = 40;
const MAX_BACKUPS: i64 = 30;
/// Repeated small edits share one backup within this many seconds.
const EDIT_BACKUP_WINDOW: i64 = 5 * 60;
const EDIT_REASON: &str = "Before editing flags";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredProfile {
    id: String,
    name: String,
    flags: Vec<FlagEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FlagDoc {
    active: String,
    profiles: Vec<StoredProfile>,
}

impl Default for FlagDoc {
    fn default() -> Self {
        Self {
            active: DEFAULT_ID.to_string(),
            profiles: vec![StoredProfile {
                id: CUSTOM_ID.to_string(),
                name: "Custom".to_string(),
                flags: Vec::new(),
            }],
        }
    }
}

impl FlagDoc {
    fn has_profile(&self, id: &str) -> bool {
        presets::find(id).is_some() || self.profiles.iter().any(|profile| profile.id == id)
    }

    fn flags_of(&self, id: &str) -> Option<Vec<FlagEntry>> {
        presets::find(id)
            .map(presets::Preset::entries)
            .or_else(|| self.profiles.iter().find(|profile| profile.id == id).map(|profile| profile.flags.clone()))
    }

    fn editable(&mut self, id: &str) -> Result<&mut StoredProfile, FlagError> {
        if presets::find(id).is_some() {
            return Err(FlagError::ReadOnlyProfile);
        }
        self.profiles
            .iter_mut()
            .find(|profile| profile.id == id)
            .ok_or(FlagError::ProfileNotFound)
    }

    fn check_new_name(&self, raw: &str, ignoring: Option<&str>) -> Result<String, FlagError> {
        let name = raw.trim();
        let length = name.chars().count();
        if length == 0 || length > MAX_PROFILE_NAME || name.chars().any(char::is_control) {
            return Err(FlagError::InvalidProfileName);
        }
        let taken = PRESETS.iter().any(|preset| preset.name.eq_ignore_ascii_case(name))
            || self
                .profiles
                .iter()
                .any(|profile| Some(profile.id.as_str()) != ignoring && profile.name.to_lowercase() == name.to_lowercase());
        if taken {
            return Err(FlagError::ProfileNameTaken);
        }
        Ok(name.to_string())
    }
}

/// When a change should be snapshotted first.
#[derive(Clone, Copy)]
enum Backup {
    Skip,
    /// Small edits, grouped so backups aren't flooded.
    Edit,
    Always(&'static str),
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

fn position(flags: &[FlagEntry], name: &str) -> Option<usize> {
    flags.iter().position(|flag| flag.name.eq_ignore_ascii_case(name))
}

/// A corrupt document is an error rather than empty, so it is never overwritten.
fn load(connection: &Connection) -> AppResult<FlagDoc> {
    let stored: Option<String> = connection
        .query_row("SELECT document FROM fflag_state WHERE id = 1", [], |row| row.get(0))
        .optional()?;
    match stored {
        Some(text) => Ok(serde_json::from_str(&text)?),
        None => Ok(FlagDoc::default()),
    }
}

fn save(connection: &Connection, document: &str) -> AppResult<()> {
    connection.execute(
        "INSERT INTO fflag_state (id, document) VALUES (1, ?1)
         ON CONFLICT (id) DO UPDATE SET document = excluded.document",
        [document],
    )?;
    Ok(())
}

fn snapshot(connection: &Connection, document: &str, backup: Backup, now: i64) -> AppResult<()> {
    let reason = match backup {
        Backup::Skip => return Ok(()),
        Backup::Edit => EDIT_REASON,
        Backup::Always(reason) => reason,
    };
    let latest: Option<(i64, String, String)> = connection
        .query_row(
            "SELECT created_at, reason, document FROM fflag_backups ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    if let Some((created_at, last_reason, last_document)) = latest {
        let same_edit_burst =
            matches!(backup, Backup::Edit) && last_reason == EDIT_REASON && now - created_at < EDIT_BACKUP_WINDOW;
        if last_document == document || same_edit_burst {
            return Ok(());
        }
    }
    connection.execute(
        "INSERT INTO fflag_backups (created_at, reason, document) VALUES (?1, ?2, ?3)",
        params![now, reason, document],
    )?;
    connection.execute(
        "DELETE FROM fflag_backups WHERE id NOT IN (SELECT id FROM fflag_backups ORDER BY id DESC LIMIT ?1)",
        [MAX_BACKUPS],
    )?;
    Ok(())
}

fn sorted_views(flags: &[FlagEntry]) -> Vec<FlagView> {
    let mut views: Vec<FlagView> = flags.iter().map(view).collect();
    views.sort_by_key(|flag| flag.name.to_ascii_lowercase());
    views
}

fn list_backups(connection: &Connection) -> AppResult<Vec<BackupInfo>> {
    let mut statement =
        connection.prepare("SELECT id, created_at, reason, document FROM fflag_backups ORDER BY id DESC")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    let mut backups = Vec::new();
    for row in rows {
        let (id, created_at, reason, document) = row?;
        // An unreadable snapshot can't be restored, so it isn't offered.
        let Ok(doc) = serde_json::from_str::<FlagDoc>(&document) else {
            log::warn!("skipping unreadable flag backup {id}");
            continue;
        };
        backups.push(BackupInfo {
            id,
            created_at: created_at as f64,
            reason,
            profile_count: doc.profiles.len() as u32,
            flag_count: doc.profiles.iter().map(|profile| profile.flags.len() as u32).sum(),
        });
    }
    Ok(backups)
}

fn overview_of(connection: &Connection, doc: &FlagDoc) -> AppResult<FlagOverview> {
    let presets = PRESETS.iter().map(|preset| ProfileView {
        id: preset.id.to_string(),
        name: preset.name.to_string(),
        source: ProfileSource::Preset,
        flags: sorted_views(&preset.entries()),
    });
    let users = doc.profiles.iter().map(|profile| ProfileView {
        id: profile.id.clone(),
        name: profile.name.clone(),
        source: ProfileSource::User,
        flags: sorted_views(&profile.flags),
    });
    Ok(FlagOverview {
        active_id: if doc.has_profile(&doc.active) { doc.active.clone() } else { DEFAULT_ID.to_string() },
        profiles: presets.chain(users).collect(),
        backups: list_backups(connection)?,
    })
}

fn render(flags: &[FlagEntry]) -> AppResult<String> {
    let map: Map<String, serde_json::Value> = flags
        .iter()
        .filter(|flag| flag.enabled)
        .map(|flag| (flag.name.clone(), to_json(flag)))
        .collect();
    Ok(serde_json::to_string_pretty(&map)?)
}

fn refuse_link(path: &Path) -> AppResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(FlagError::UnsafeTarget.into()),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Replaces the settings file in one step, so Roblox never reads half a file.
fn write_client_settings(install: &Path, contents: &str) -> AppResult<()> {
    let folder = install.join("ClientSettings");
    if refuse_link(&folder)? {
        if !folder.is_dir() {
            return Err(FlagError::UnsafeTarget.into());
        }
    } else {
        fs::create_dir(&folder)?;
    }
    let target = folder.join("ClientAppSettings.json");
    refuse_link(&target)?;

    let temporary = folder.join(format!(".ClientAppSettings-{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temporary, contents)?;
    if let Err(error) = fs::rename(&temporary, &target) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

pub struct FlagManager {
    db: Arc<Database>,
}

impl FlagManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn overview(&self) -> AppResult<FlagOverview> {
        let connection = self.db.connection();
        let doc = load(&connection)?;
        overview_of(&connection, &doc)
    }

    /// Runs one change in a transaction, backing up the previous state first.
    fn mutate(
        &self,
        backup: Backup,
        change: impl FnOnce(&mut FlagDoc) -> Result<(), FlagError>,
    ) -> AppResult<FlagOverview> {
        let mut connection = self.db.connection();
        let transaction = connection.transaction()?;
        let mut doc = load(&transaction)?;
        let before = serde_json::to_string(&doc)?;
        change(&mut doc)?;
        let after = serde_json::to_string(&doc)?;
        if before != after {
            snapshot(&transaction, &before, backup, unix_now())?;
            save(&transaction, &after)?;
        }
        transaction.commit()?;
        overview_of(&connection, &doc)
    }

    pub fn set_active(&self, id: &str) -> AppResult<FlagOverview> {
        self.mutate(Backup::Skip, |doc| {
            if !doc.has_profile(id) {
                return Err(FlagError::ProfileNotFound);
            }
            doc.active = id.to_string();
            Ok(())
        })
    }

    pub fn create_profile(&self, name: &str, copy_from: Option<&str>) -> AppResult<FlagOverview> {
        self.mutate(Backup::Skip, |doc| {
            if doc.profiles.len() >= MAX_PROFILES {
                return Err(FlagError::TooManyProfiles);
            }
            let name = doc.check_new_name(name, None)?;
            let flags = match copy_from {
                Some(source) => doc.flags_of(source).ok_or(FlagError::ProfileNotFound)?,
                None => Vec::new(),
            };
            let id = uuid::Uuid::new_v4().to_string();
            doc.profiles.push(StoredProfile { id: id.clone(), name, flags });
            doc.active = id;
            Ok(())
        })
    }

    pub fn rename_profile(&self, id: &str, name: &str) -> AppResult<FlagOverview> {
        self.mutate(Backup::Edit, |doc| {
            doc.editable(id)?;
            let name = doc.check_new_name(name, Some(id))?;
            doc.editable(id)?.name = name;
            Ok(())
        })
    }

    pub fn delete_profile(&self, id: &str) -> AppResult<FlagOverview> {
        self.mutate(Backup::Always("Before deleting a profile"), |doc| {
            doc.editable(id)?;
            doc.profiles.retain(|profile| profile.id != id);
            if doc.active == id {
                doc.active = DEFAULT_ID.to_string();
            }
            Ok(())
        })
    }

    /// Adds a flag, or edits the one named `original`.
    pub fn save_flag(&self, profile_id: &str, original: Option<&str>, input: FlagInput) -> AppResult<FlagOverview> {
        let name = check_name(&input.name)?;
        let value = normalize_value(&name, input.kind, &input.value)?;
        let entry = FlagEntry { name, kind: input.kind, value, enabled: input.enabled };

        self.mutate(Backup::Edit, |doc| {
            let profile = doc.editable(profile_id)?;
            let existing = position(&profile.flags, &entry.name);
            match original {
                Some(original) => {
                    let index = position(&profile.flags, original).ok_or(FlagError::FlagNotFound)?;
                    if existing.is_some_and(|found| found != index) {
                        return Err(FlagError::Duplicate(entry.name));
                    }
                    profile.flags[index] = entry;
                }
                None => {
                    if existing.is_some() {
                        return Err(FlagError::Duplicate(entry.name));
                    }
                    if profile.flags.len() >= MAX_FLAGS {
                        return Err(FlagError::TooLarge);
                    }
                    profile.flags.push(entry);
                }
            }
            Ok(())
        })
    }

    pub fn set_flag_enabled(&self, profile_id: &str, name: &str, enabled: bool) -> AppResult<FlagOverview> {
        self.mutate(Backup::Edit, |doc| {
            let profile = doc.editable(profile_id)?;
            let index = position(&profile.flags, name).ok_or(FlagError::FlagNotFound)?;
            profile.flags[index].enabled = enabled;
            Ok(())
        })
    }

    pub fn remove_flag(&self, profile_id: &str, name: &str) -> AppResult<FlagOverview> {
        self.mutate(Backup::Edit, |doc| {
            let profile = doc.editable(profile_id)?;
            let index = position(&profile.flags, name).ok_or(FlagError::FlagNotFound)?;
            profile.flags.remove(index);
            Ok(())
        })
    }

    /// Replaces the profile's flags, or merges with imported values winning.
    pub fn import(&self, profile_id: &str, text: &str, replace: bool) -> AppResult<FlagOverview> {
        let imported = parse_import(text)?;
        self.mutate(Backup::Always("Before import"), |doc| {
            let profile = doc.editable(profile_id)?;
            if replace {
                profile.flags = imported;
            } else {
                for entry in imported {
                    match position(&profile.flags, &entry.name) {
                        Some(index) => profile.flags[index] = entry,
                        None => profile.flags.push(entry),
                    }
                }
                if profile.flags.len() > MAX_FLAGS {
                    return Err(FlagError::TooLarge);
                }
            }
            Ok(())
        })
    }

    /// Enabled flags as `ClientAppSettings.json` content.
    pub fn export_json(&self, profile_id: &str) -> AppResult<String> {
        let doc = load(&self.db.connection())?;
        let flags = doc.flags_of(profile_id).ok_or(FlagError::ProfileNotFound)?;
        render(&flags)
    }

    pub fn profile_name(&self, profile_id: &str) -> AppResult<String> {
        if let Some(preset) = presets::find(profile_id) {
            return Ok(preset.name.to_string());
        }
        let doc = load(&self.db.connection())?;
        doc.profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .map(|profile| profile.name)
            .ok_or_else(|| FlagError::ProfileNotFound.into())
    }

    pub fn reset(&self) -> AppResult<FlagOverview> {
        self.mutate(Backup::Always("Before reset"), |doc| {
            *doc = FlagDoc::default();
            Ok(())
        })
    }

    pub fn restore_backup(&self, id: i64) -> AppResult<FlagOverview> {
        let document: String = self
            .db
            .connection()
            .query_row("SELECT document FROM fflag_backups WHERE id = ?1", [id], |row| row.get(0))
            .optional()?
            .ok_or(FlagError::BackupNotFound)?;
        let restored: FlagDoc = serde_json::from_str(&document)?;
        self.mutate(Backup::Always("Before restoring a backup"), |doc| {
            *doc = restored;
            Ok(())
        })
    }

    /// Writes the active profile into a BloxClient-installed version.
    pub fn apply_to(&self, install: &Path) -> AppResult<()> {
        let doc = load(&self.db.connection())?;
        let active = if doc.has_profile(&doc.active) { doc.active.as_str() } else { DEFAULT_ID };
        let flags = doc.flags_of(active).unwrap_or_default();
        write_client_settings(install, &render(&flags)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use std::path::PathBuf;

    fn manager() -> FlagManager {
        FlagManager::new(Arc::new(Database::in_memory().unwrap()))
    }

    fn input(name: &str, kind: FlagKind, value: &str) -> FlagInput {
        FlagInput { name: name.into(), kind, value: value.into(), enabled: true }
    }

    fn flags<'a>(overview: &'a FlagOverview, id: &str) -> &'a [FlagView] {
        &overview.profiles.iter().find(|profile| profile.id == id).unwrap().flags
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bloxclient-flags-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn starts_on_the_empty_default_profile() {
        let overview = manager().overview().unwrap();
        assert_eq!(overview.active_id, DEFAULT_ID);
        assert!(flags(&overview, DEFAULT_ID).is_empty());
        assert_eq!(overview.profiles.len(), 4);
    }

    #[test]
    fn presets_cannot_be_changed() {
        let manager = manager();
        let result = manager.save_flag("preset:quality", None, input("FFlagA", FlagKind::Boolean, "true"));
        assert!(matches!(result, Err(AppError::FFlags(FlagError::ReadOnlyProfile))));
        assert!(matches!(
            manager.delete_profile("preset:quality"),
            Err(AppError::FFlags(FlagError::ReadOnlyProfile))
        ));
        assert!(matches!(
            manager.import("preset:quality", "{}", true),
            Err(AppError::FFlags(FlagError::ReadOnlyProfile))
        ));
    }

    #[test]
    fn duplicating_a_preset_gives_an_editable_active_copy() {
        let manager = manager();
        let overview = manager.create_profile("My quality", Some("preset:quality")).unwrap();
        let copy = overview.profiles.last().unwrap();
        assert_eq!(copy.source, ProfileSource::User);
        assert_eq!(copy.flags.len(), 3);
        assert_eq!(overview.active_id, copy.id);
    }

    #[test]
    fn duplicate_flags_and_profile_names_are_refused() {
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FFlagA", FlagKind::Boolean, "true")).unwrap();
        assert!(matches!(
            manager.save_flag(CUSTOM_ID, None, input("fflaga", FlagKind::Boolean, "false")),
            Err(AppError::FFlags(FlagError::Duplicate(_)))
        ));
        manager.save_flag(CUSTOM_ID, None, input("FFlagB", FlagKind::Boolean, "true")).unwrap();
        assert!(matches!(
            manager.save_flag(CUSTOM_ID, Some("FFlagB"), input("FFlagA", FlagKind::Boolean, "true")),
            Err(AppError::FFlags(FlagError::Duplicate(_)))
        ));
        for name in ["custom", "PERFORMANCE", "   "] {
            assert!(manager.create_profile(name, None).is_err(), "accepted {name:?}");
        }
    }

    #[test]
    fn editing_can_rename_a_flag() {
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FIntA", FlagKind::Number, "1")).unwrap();
        let overview = manager
            .save_flag(CUSTOM_ID, Some("FIntA"), input("FIntB", FlagKind::Number, "2"))
            .unwrap();
        let list = flags(&overview, CUSTOM_ID);
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].name.as_str(), list[0].value.as_str()), ("FIntB", "2"));
    }

    #[test]
    fn export_leaves_out_disabled_flags_and_keeps_types() {
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FFlagOn", FlagKind::Boolean, "true")).unwrap();
        manager.save_flag(CUSTOM_ID, None, input("FIntNum", FlagKind::Number, "5")).unwrap();
        manager.save_flag(CUSTOM_ID, None, input("FFlagOff", FlagKind::Boolean, "true")).unwrap();
        manager.set_flag_enabled(CUSTOM_ID, "FFlagOff", false).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&manager.export_json(CUSTOM_ID).unwrap()).unwrap();
        assert_eq!(parsed, serde_json::json!({ "FFlagOn": true, "FIntNum": 5 }));
    }

    #[test]
    fn import_can_merge_or_replace() {
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FIntKeep", FlagKind::Number, "1")).unwrap();
        manager.save_flag(CUSTOM_ID, None, input("FIntSame", FlagKind::Number, "1")).unwrap();

        let merged = manager.import(CUSTOM_ID, "{\"fintsame\":2,\"FIntNew\":3}", false).unwrap();
        assert_eq!(flags(&merged, CUSTOM_ID).len(), 3);

        let replaced = manager.import(CUSTOM_ID, "{\"FIntOnly\":9}", true).unwrap();
        assert_eq!(flags(&replaced, CUSTOM_ID).len(), 1);
    }

    #[test]
    fn failed_import_changes_nothing() {
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FIntKeep", FlagKind::Number, "1")).unwrap();
        let before = manager.overview().unwrap().backups.len();
        assert!(manager.import(CUSTOM_ID, "{\"FIntA\":1,\"FIntA\":2}", true).is_err());
        let after = manager.overview().unwrap();
        assert_eq!(flags(&after, CUSTOM_ID).len(), 1);
        assert_eq!(after.backups.len(), before);
    }

    #[test]
    fn backups_restore_earlier_state_and_are_undoable() {
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FIntA", FlagKind::Number, "1")).unwrap();
        let overview = manager.reset().unwrap();
        assert!(flags(&overview, CUSTOM_ID).is_empty());
        let backup = overview.backups.iter().find(|item| item.reason == "Before reset").unwrap().id;

        let restored = manager.restore_backup(backup).unwrap();
        assert_eq!(flags(&restored, CUSTOM_ID).len(), 1);
        assert_eq!(restored.backups[0].reason, "Before restoring a backup");
        assert!(matches!(
            manager.restore_backup(9_999),
            Err(AppError::FFlags(FlagError::BackupNotFound))
        ));
    }

    #[test]
    fn small_edits_share_a_backup_and_old_backups_are_trimmed() {
        let db = Database::in_memory().unwrap();
        let connection = db.connection();
        let count = |connection: &Connection| -> i64 {
            connection.query_row("SELECT count(*) FROM fflag_backups", [], |row| row.get(0)).unwrap()
        };
        snapshot(&connection, "a", Backup::Edit, 1_000).unwrap();
        snapshot(&connection, "b", Backup::Edit, 1_000 + EDIT_BACKUP_WINDOW - 1).unwrap();
        assert_eq!(count(&connection), 1);
        snapshot(&connection, "c", Backup::Edit, 1_000 + EDIT_BACKUP_WINDOW).unwrap();
        assert_eq!(count(&connection), 2);
        snapshot(&connection, "c", Backup::Always("Before import"), 5_000).unwrap();
        assert_eq!(count(&connection), 2, "identical state isn't stored twice");

        for index in 0..(MAX_BACKUPS + 10) {
            snapshot(&connection, &format!("doc{index}"), Backup::Always("Before import"), 6_000).unwrap();
        }
        assert_eq!(count(&connection), MAX_BACKUPS);
    }

    #[test]
    fn corrupt_state_is_an_error_and_is_not_overwritten() {
        let manager = manager();
        manager
            .db
            .connection()
            .execute("INSERT INTO fflag_state (id, document) VALUES (1, 'not json')", [])
            .unwrap();
        assert!(manager.set_active("preset:quality").is_err());
        let kept: String = manager
            .db
            .connection()
            .query_row("SELECT document FROM fflag_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kept, "not json");
    }

    #[test]
    fn apply_writes_the_active_profile_only() {
        let install = temp_dir("apply");
        let manager = manager();
        manager.save_flag(CUSTOM_ID, None, input("FIntA", FlagKind::Number, "7")).unwrap();
        manager.set_active(CUSTOM_ID).unwrap();
        manager.apply_to(&install).unwrap();

        let path = install.join("ClientSettings").join("ClientAppSettings.json");
        let written: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written, serde_json::json!({ "FIntA": 7 }));

        manager.set_active(DEFAULT_ID).unwrap();
        manager.apply_to(&install).unwrap();
        let cleared: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(cleared, serde_json::json!({}));

        let leftovers = fs::read_dir(install.join("ClientSettings")).unwrap().count();
        fs::remove_dir_all(&install).unwrap();
        assert_eq!(leftovers, 1, "no temporary files should remain");
    }

    #[test]
    fn apply_refuses_a_file_where_the_folder_should_be() {
        let install = temp_dir("blocked");
        fs::write(install.join("ClientSettings"), b"x").unwrap();
        let result = manager().apply_to(&install);
        let kept = fs::read(install.join("ClientSettings")).unwrap();
        fs::remove_dir_all(&install).unwrap();
        assert!(matches!(result, Err(AppError::FFlags(FlagError::UnsafeTarget))));
        assert_eq!(kept, b"x");
    }
}
