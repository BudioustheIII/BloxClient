use std::sync::{Arc, Mutex, PoisonError, RwLock};

use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use super::model::Settings;
use crate::db::Database;
use crate::error::{AppError, AppResult};

const SETTINGS_KEY: &str = "app";
const QUARANTINE_KEY: &str = "app.unreadable";
const LAST_VERSION_KEY: &str = "app.last_version";

/// Validated settings with a read cache. Every change is persisted before the
/// cache updates, and native side effects are undone if persistence fails.
pub struct SettingsStore {
    db: Arc<Database>,
    current: RwLock<Settings>,
    write_lock: Mutex<()>,
}

impl SettingsStore {
    pub fn load(db: Arc<Database>) -> AppResult<Self> {
        let stored: Option<String> = db
            .connection()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [SETTINGS_KEY],
                |row| row.get(0),
            )
            .optional()?;

        let settings = match stored {
            Some(raw) => match parse_stored(&raw) {
                Ok(settings) => settings,
                Err(error) => {
                    // Keep the unreadable value so it can be inspected or recovered.
                    log::warn!("stored settings were unreadable, using defaults: {error}");
                    write_row(&db, QUARANTINE_KEY, &raw)?;
                    let defaults = Settings::default();
                    write_row(&db, SETTINGS_KEY, &serde_json::to_string(&defaults)?)?;
                    defaults
                }
            },
            None => {
                let defaults = Settings::default();
                write_row(&db, SETTINGS_KEY, &serde_json::to_string(&defaults)?)?;
                defaults
            }
        };

        Ok(Self {
            db,
            current: RwLock::new(settings),
            write_lock: Mutex::new(()),
        })
    }

    pub fn get(&self) -> Settings {
        self.current
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The app version that last finished startup checks on this profile.
    pub fn last_run_version(&self) -> AppResult<Option<String>> {
        let stored: Option<String> = self
            .db
            .connection()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [LAST_VERSION_KEY],
                |row| row.get(0),
            )
            .optional()?;
        Ok(stored.and_then(|raw| serde_json::from_str(&raw).ok()))
    }

    pub fn set_last_run_version(&self, version: &str) -> AppResult<()> {
        write_row(&self.db, LAST_VERSION_KEY, &serde_json::to_string(version)?)
    }

    /// Applies a partial update. `side_effects` receives (previous, next).
    pub fn update(
        &self,
        patch: &Value,
        side_effects: impl Fn(&Settings, &Settings) -> AppResult<()>,
    ) -> AppResult<Settings> {
        let _guard = self.write_lock.lock().unwrap_or_else(PoisonError::into_inner);
        let previous = self.get();
        let next = apply_patch(&previous, patch)?;
        self.commit(previous, next, side_effects)
    }

    pub fn reset(
        &self,
        side_effects: impl Fn(&Settings, &Settings) -> AppResult<()>,
    ) -> AppResult<Settings> {
        let _guard = self.write_lock.lock().unwrap_or_else(PoisonError::into_inner);
        self.commit(self.get(), Settings::default(), side_effects)
    }

    fn commit(
        &self,
        previous: Settings,
        next: Settings,
        side_effects: impl Fn(&Settings, &Settings) -> AppResult<()>,
    ) -> AppResult<Settings> {
        if previous == next {
            return Ok(next);
        }
        side_effects(&previous, &next)?;

        let persisted = serde_json::to_string(&next)
            .map_err(AppError::from)
            .and_then(|json| write_row(&self.db, SETTINGS_KEY, &json));
        if let Err(error) = persisted {
            if let Err(undo) = side_effects(&next, &previous) {
                log::error!("could not undo settings side effects: {undo}");
            }
            return Err(error);
        }

        *self.current.write().unwrap_or_else(PoisonError::into_inner) = next.clone();
        Ok(next)
    }
}

fn parse_stored(raw: &str) -> AppResult<Settings> {
    let settings: Settings = serde_json::from_str(raw)?;
    settings.validate()?;
    Ok(settings)
}

fn write_row(db: &Database, key: &str, value: &str) -> AppResult<()> {
    db.connection().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
        params![key, value],
    )?;
    Ok(())
}

fn apply_patch(current: &Settings, patch: &Value) -> AppResult<Settings> {
    let mut merged = serde_json::to_value(current)?;
    merge(&mut merged, patch, "")?;
    let next: Settings = serde_json::from_value(merged)
        .map_err(|error| AppError::InvalidSettings(error.to_string()))?;
    next.validate()?;
    Ok(next)
}

/// Merges `patch` into `target`, rejecting keys the schema doesn't define so a
/// mistyped field fails loudly instead of being silently dropped.
fn merge(target: &mut Value, patch: &Value, path: &str) -> AppResult<()> {
    let (Value::Object(target), Value::Object(patch)) = (target, patch) else {
        return Err(AppError::InvalidSettings(format!(
            "expected an object at '{path}'"
        )));
    };

    for (key, value) in patch {
        let field = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        let existing = target
            .get_mut(key)
            .ok_or_else(|| AppError::InvalidSettings(format!("unknown setting '{field}'")))?;
        if existing.is_object() {
            merge(existing, value, &field)?;
        } else {
            *existing = value.clone();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::model::{PageId, Theme};
    use super::*;
    use serde_json::json;
    use std::cell::Cell;

    fn store() -> (Arc<Database>, SettingsStore) {
        let db = Arc::new(Database::in_memory().unwrap());
        let store = SettingsStore::load(Arc::clone(&db)).unwrap();
        (db, store)
    }

    fn no_effects(_: &Settings, _: &Settings) -> AppResult<()> {
        Ok(())
    }

    #[test]
    fn first_load_persists_defaults() {
        let (db, store) = store();
        assert_eq!(store.get(), Settings::default());
        let reloaded = SettingsStore::load(db).unwrap();
        assert_eq!(reloaded.get(), Settings::default());
    }

    #[test]
    fn partial_update_survives_reload_and_keeps_other_fields() {
        let (db, store) = store();
        store
            .update(
                &json!({ "appearance": { "theme": "light" }, "general": { "defaultPage": "friends" } }),
                no_effects,
            )
            .unwrap();

        let reloaded = SettingsStore::load(db).unwrap().get();
        assert_eq!(reloaded.appearance.theme, Theme::Light);
        assert_eq!(reloaded.general.default_page, PageId::Friends);
        assert_eq!(reloaded.appearance.ui_scale, 100);
    }

    #[test]
    fn rejects_unknown_keys_bad_types_and_bad_values() {
        let (_db, store) = store();
        for patch in [
            json!({ "appearance": { "themes": "light" } }),
            json!({ "nope": true }),
            json!({ "appearance": { "theme": "neon" } }),
            json!({ "appearance": "dark" }),
            json!({ "appearance": { "accent": "red" } }),
            json!({ "appearance": { "accent": "#12345G" } }),
            json!({ "appearance": { "uiScale": 500 } }),
            json!({ "general": { "minimizeToTray": null } }),
        ] {
            assert!(store.update(&patch, no_effects).is_err(), "accepted {patch}");
        }
        assert_eq!(store.get(), Settings::default());
    }

    #[test]
    fn failed_side_effect_changes_nothing() {
        let (db, store) = store();
        let result = store.update(&json!({ "general": { "launchOnStartup": true } }), |_, _| {
            Err(AppError::InvalidSettings("denied".into()))
        });
        assert!(result.is_err());
        assert!(!store.get().general.launch_on_startup);
        assert!(!SettingsStore::load(db).unwrap().get().general.launch_on_startup);
    }

    #[test]
    fn failed_persist_undoes_side_effects() {
        let (db, store) = store();
        db.connection()
            .execute_batch("DROP TABLE settings")
            .unwrap();

        let calls = Cell::new(0);
        let result = store.update(&json!({ "general": { "launchOnStartup": true } }), |_, _| {
            calls.set(calls.get() + 1);
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(calls.get(), 2, "effect should run, then be undone");
        assert!(!store.get().general.launch_on_startup);
    }

    #[test]
    fn unreadable_settings_are_quarantined() {
        let db = Arc::new(Database::in_memory().unwrap());
        let bad = r##"{"appearance":{"accent":"not-a-color"}}"##;
        write_row(&db, SETTINGS_KEY, bad).unwrap();

        let store = SettingsStore::load(Arc::clone(&db)).unwrap();
        assert_eq!(store.get(), Settings::default());
        let kept: String = db
            .connection()
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [QUARANTINE_KEY],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, bad);
    }

    #[test]
    fn reset_restores_defaults() {
        let (_db, store) = store();
        store
            .update(&json!({ "appearance": { "uiScale": 125 } }), no_effects)
            .unwrap();
        assert_eq!(store.reset(no_effects).unwrap(), Settings::default());
    }
}
