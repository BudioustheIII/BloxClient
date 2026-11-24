//! The personal game library: favorites, pins and collections. Stored only on
//! this PC and encrypted, because Roblox account favorites require sign-in.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::AppResult;
use crate::roblox::history::{bounded, trusted_image};
use crate::roblox::LaunchTarget;
use crate::vault;

const VAULT_KEY: &str = "library";
const MAX_GAMES: usize = 2_000;
const MAX_COLLECTIONS: usize = 50;
const MAX_COLLECTION_NAME: usize = 40;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("invalid game")]
    InvalidGame,
    #[error("collection not found")]
    CollectionNotFound,
    #[error("invalid collection name")]
    InvalidName,
    #[error("collection name already used")]
    NameTaken,
    #[error("library limit reached")]
    Full,
}

impl LibraryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidGame => "library_invalid_game",
            Self::CollectionNotFound => "library_collection_not_found",
            Self::InvalidName => "library_invalid_name",
            Self::NameTaken => "library_name_taken",
            Self::Full => "library_full",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidGame => "That game couldn't be saved.".into(),
            Self::CollectionNotFound => "That collection no longer exists.".into(),
            Self::InvalidName => "Collection names must be 1 to 40 characters.".into(),
            Self::NameTaken => "You already have a collection with that name.".into(),
            Self::Full => "Your library is full. Remove something to add more.".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SavedGame {
    universe_id: u64,
    place_id: u64,
    name: String,
    creator_name: Option<String>,
    thumbnail_url: Option<String>,
    added_at: i64,
    favorite: bool,
    pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Collection {
    id: String,
    name: String,
    game_ids: Vec<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct LibraryDoc {
    games: Vec<SavedGame>,
    collections: Vec<Collection>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct LibraryGame {
    #[cfg_attr(test, ts(type = "number"))]
    pub universe_id: u64,
    #[cfg_attr(test, ts(type = "number"))]
    pub place_id: u64,
    pub name: String,
    pub creator_name: Option<String>,
    pub thumbnail_url: Option<String>,
    pub added_at: f64,
    pub favorite: bool,
    pub pinned: bool,
    pub collection_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct LibraryCollection {
    pub id: String,
    pub name: String,
    pub game_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct LibraryOverview {
    pub games: Vec<LibraryGame>,
    pub collections: Vec<LibraryCollection>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

impl LibraryDoc {
    /// Finds the game, adding it if needed, and refreshes its saved details.
    fn upsert(&mut self, target: &LaunchTarget, now: i64) -> Result<&mut SavedGame, LibraryError> {
        if target.universe_id == 0 || target.place_id == 0 || target.name.trim().is_empty() {
            return Err(LibraryError::InvalidGame);
        }
        let index = match self.games.iter().position(|game| game.universe_id == target.universe_id) {
            Some(index) => index,
            None => {
                if self.games.len() >= MAX_GAMES {
                    return Err(LibraryError::Full);
                }
                self.games.push(SavedGame {
                    universe_id: target.universe_id,
                    place_id: target.place_id,
                    name: String::new(),
                    creator_name: None,
                    thumbnail_url: None,
                    added_at: now,
                    favorite: false,
                    pinned: false,
                });
                self.games.len() - 1
            }
        };
        let game = &mut self.games[index];
        game.place_id = target.place_id;
        game.name = bounded(&target.name);
        if let Some(creator) = &target.creator_name {
            game.creator_name = Some(bounded(creator));
        }
        if let Some(url) = trusted_image(target.thumbnail_url.as_deref()) {
            game.thumbnail_url = Some(url);
        }
        Ok(game)
    }

    /// Forgets games that are no longer favorited, pinned or collected.
    fn prune(&mut self) {
        let collections = &self.collections;
        self.games.retain(|game| {
            game.favorite
                || game.pinned
                || collections.iter().any(|collection| collection.game_ids.contains(&game.universe_id))
        });
    }

    fn set_favorite(&mut self, target: &LaunchTarget, favorite: bool, now: i64) -> Result<(), LibraryError> {
        self.upsert(target, now)?.favorite = favorite;
        self.prune();
        Ok(())
    }

    fn set_pinned(&mut self, target: &LaunchTarget, pinned: bool, now: i64) -> Result<(), LibraryError> {
        self.upsert(target, now)?.pinned = pinned;
        self.prune();
        Ok(())
    }

    fn check_name(&self, raw: &str, ignoring: Option<&str>) -> Result<String, LibraryError> {
        let name = raw.trim();
        let length = name.chars().count();
        if length == 0 || length > MAX_COLLECTION_NAME || name.chars().any(char::is_control) {
            return Err(LibraryError::InvalidName);
        }
        let lower = name.to_lowercase();
        let taken = self
            .collections
            .iter()
            .any(|item| Some(item.id.as_str()) != ignoring && item.name.to_lowercase() == lower);
        if taken {
            return Err(LibraryError::NameTaken);
        }
        Ok(name.to_string())
    }

    fn create_collection(&mut self, name: &str) -> Result<(), LibraryError> {
        if self.collections.len() >= MAX_COLLECTIONS {
            return Err(LibraryError::Full);
        }
        let name = self.check_name(name, None)?;
        self.collections.push(Collection {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            game_ids: Vec::new(),
        });
        Ok(())
    }

    fn rename_collection(&mut self, id: &str, name: &str) -> Result<(), LibraryError> {
        let name = self.check_name(name, Some(id))?;
        self.collection_mut(id)?.name = name;
        Ok(())
    }

    fn delete_collection(&mut self, id: &str) -> Result<(), LibraryError> {
        self.collection_mut(id)?;
        self.collections.retain(|item| item.id != id);
        self.prune();
        Ok(())
    }

    fn collection_mut(&mut self, id: &str) -> Result<&mut Collection, LibraryError> {
        self.collections
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or(LibraryError::CollectionNotFound)
    }

    fn set_in_collection(
        &mut self,
        id: &str,
        target: &LaunchTarget,
        included: bool,
        now: i64,
    ) -> Result<(), LibraryError> {
        self.collection_mut(id)?;
        self.upsert(target, now)?;
        let collection = self.collection_mut(id)?;
        collection.game_ids.retain(|game| *game != target.universe_id);
        if included {
            collection.game_ids.push(target.universe_id);
        }
        self.prune();
        Ok(())
    }

    fn overview(&self) -> LibraryOverview {
        let mut games: Vec<LibraryGame> = self
            .games
            .iter()
            .map(|game| LibraryGame {
                universe_id: game.universe_id,
                place_id: game.place_id,
                name: game.name.clone(),
                creator_name: game.creator_name.clone(),
                thumbnail_url: game.thumbnail_url.clone(),
                added_at: game.added_at as f64,
                favorite: game.favorite,
                pinned: game.pinned,
                collection_ids: self
                    .collections
                    .iter()
                    .filter(|item| item.game_ids.contains(&game.universe_id))
                    .map(|item| item.id.clone())
                    .collect(),
            })
            .collect();
        games.sort_by(|a, b| b.added_at.total_cmp(&a.added_at).then(a.universe_id.cmp(&b.universe_id)));

        let mut collections: Vec<LibraryCollection> = self
            .collections
            .iter()
            .map(|item| LibraryCollection {
                id: item.id.clone(),
                name: item.name.clone(),
                game_count: item.game_ids.len() as u32,
            })
            .collect();
        collections.sort_by_key(|item| item.name.to_lowercase());
        LibraryOverview { games, collections }
    }
}

pub struct Library {
    db: Arc<Database>,
}

impl Library {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn overview(&self) -> AppResult<LibraryOverview> {
        let doc: LibraryDoc = vault::read(&self.db.connection(), VAULT_KEY)?.unwrap_or_default();
        Ok(doc.overview())
    }

    fn update(&self, change: impl FnOnce(&mut LibraryDoc) -> Result<(), LibraryError>) -> AppResult<LibraryOverview> {
        let connection = self.db.connection();
        let mut doc: LibraryDoc = vault::read(&connection, VAULT_KEY)?.unwrap_or_default();
        change(&mut doc)?;
        vault::write(&connection, VAULT_KEY, &doc)?;
        Ok(doc.overview())
    }

    pub fn set_favorite(&self, target: &LaunchTarget, favorite: bool) -> AppResult<LibraryOverview> {
        self.update(|doc| doc.set_favorite(target, favorite, unix_now()))
    }

    pub fn set_pinned(&self, target: &LaunchTarget, pinned: bool) -> AppResult<LibraryOverview> {
        self.update(|doc| doc.set_pinned(target, pinned, unix_now()))
    }

    pub fn create_collection(&self, name: &str) -> AppResult<LibraryOverview> {
        self.update(|doc| doc.create_collection(name))
    }

    pub fn rename_collection(&self, id: &str, name: &str) -> AppResult<LibraryOverview> {
        self.update(|doc| doc.rename_collection(id, name))
    }

    pub fn delete_collection(&self, id: &str) -> AppResult<LibraryOverview> {
        self.update(|doc| doc.delete_collection(id))
    }

    pub fn set_in_collection(&self, id: &str, target: &LaunchTarget, included: bool) -> AppResult<LibraryOverview> {
        self.update(|doc| doc.set_in_collection(id, target, included, unix_now()))
    }

    pub fn clear(&self) -> AppResult<LibraryOverview> {
        vault::remove(&self.db.connection(), VAULT_KEY)?;
        Ok(LibraryDoc::default().overview())
    }
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
    fn favorites_and_pins_are_independent() {
        let mut doc = LibraryDoc::default();
        doc.set_favorite(&target(1, "One"), true, 10).unwrap();
        doc.set_pinned(&target(1, "One"), true, 20).unwrap();
        doc.set_favorite(&target(1, "One"), false, 30).unwrap();
        let game = &doc.overview().games[0];
        assert!(game.pinned && !game.favorite);
        assert_eq!(game.added_at, 10.0);
    }

    #[test]
    fn games_are_forgotten_when_nothing_keeps_them() {
        let mut doc = LibraryDoc::default();
        doc.set_favorite(&target(1, "One"), true, 10).unwrap();
        doc.set_favorite(&target(1, "One"), false, 20).unwrap();
        assert!(doc.games.is_empty());
    }

    #[test]
    fn unfavoriting_an_unsaved_game_leaves_nothing_behind() {
        let mut doc = LibraryDoc::default();
        doc.set_pinned(&target(9, "Nine"), false, 10).unwrap();
        assert!(doc.games.is_empty());
    }

    #[test]
    fn collections_keep_games_until_deleted() {
        let mut doc = LibraryDoc::default();
        doc.create_collection("Favorites to try").unwrap();
        let id = doc.collections[0].id.clone();
        doc.set_in_collection(&id, &target(1, "One"), true, 10).unwrap();
        doc.set_in_collection(&id, &target(1, "One"), true, 11).unwrap();
        assert_eq!(doc.overview().collections[0].game_count, 1);
        assert_eq!(doc.overview().games[0].collection_ids, [id.clone()]);

        doc.delete_collection(&id).unwrap();
        assert!(doc.games.is_empty());
    }

    #[test]
    fn deleting_a_collection_keeps_favorites() {
        let mut doc = LibraryDoc::default();
        doc.create_collection("List").unwrap();
        let id = doc.collections[0].id.clone();
        doc.set_favorite(&target(1, "One"), true, 10).unwrap();
        doc.set_in_collection(&id, &target(1, "One"), true, 11).unwrap();
        doc.delete_collection(&id).unwrap();
        assert_eq!(doc.games.len(), 1);
        assert!(doc.overview().games[0].collection_ids.is_empty());
    }

    #[test]
    fn collection_names_are_checked() {
        let mut doc = LibraryDoc::default();
        doc.create_collection("Racing").unwrap();
        assert!(matches!(doc.create_collection(" racing "), Err(LibraryError::NameTaken)));
        assert!(matches!(doc.create_collection("  "), Err(LibraryError::InvalidName)));
        assert!(matches!(doc.create_collection(&"x".repeat(41)), Err(LibraryError::InvalidName)));
        let id = doc.collections[0].id.clone();
        doc.rename_collection(&id, "RACING").unwrap();
        assert_eq!(doc.collections[0].name, "RACING");
    }

    #[test]
    fn invalid_games_and_images_are_not_saved() {
        let mut doc = LibraryDoc::default();
        assert!(doc.set_favorite(&target(0, "Zero"), true, 1).is_err());
        assert!(doc.set_favorite(&target(1, "  "), true, 1).is_err());

        let mut bad_image = target(2, "Two");
        bad_image.thumbnail_url = Some("https://evil.example/x.png".to_string());
        doc.set_favorite(&bad_image, true, 1).unwrap();
        assert_eq!(doc.games[0].thumbnail_url, None);
    }

    #[test]
    fn missing_collection_changes_nothing() {
        let mut doc = LibraryDoc::default();
        assert!(matches!(
            doc.set_in_collection("missing", &target(1, "One"), true, 1),
            Err(LibraryError::CollectionNotFound)
        ));
        assert!(doc.games.is_empty());
    }

    #[test]
    fn library_is_encrypted_and_clearable() {
        let db = Arc::new(Database::in_memory().unwrap());
        let library = Library::new(Arc::clone(&db));
        library.set_favorite(&target(1, "Very Private Game"), true).unwrap();
        let raw: Vec<u8> = db
            .connection()
            .query_row("SELECT value FROM vault WHERE key = ?1", [VAULT_KEY], |row| row.get(0))
            .unwrap();
        assert!(!raw.windows(7).any(|window| window == b"Private"));
        assert_eq!(library.overview().unwrap().games.len(), 1);
        assert!(library.clear().unwrap().games.is_empty());
        assert!(library.overview().unwrap().games.is_empty());
    }
}
