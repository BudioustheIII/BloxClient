//! Local profiles and the people each one follows.
//!
//! Roblox only gives a friends list to a signed-in session, and BloxClient
//! never holds one. So a profile is just a public username, and its people are
//! added by hand. Switching profiles here doesn't change the Roblox Player's login.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::error::RobloxError;
use super::history::bounded;
use super::http::RobloxHttp;
use super::people_api::{self, PresenceEntry, PublicUser};
use crate::db::Database;
use crate::error::AppResult;
use crate::vault;

const VAULT_KEY: &str = "people";
const MAX_PROFILES: usize = 10;
const MAX_PEOPLE: usize = 200;
const AVATAR_TTL: Duration = Duration::from_secs(30 * 60);
const COUNT_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub enum PresenceState {
    /// Offline, or hidden by the person's privacy settings.
    Offline,
    Online,
    InGame,
    InStudio,
    Invisible,
    /// Roblox couldn't be asked.
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    #[cfg_attr(test, ts(type = "number"))]
    pub user_id: u64,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub presence: PresenceState,
    pub location: Option<String>,
    #[cfg_attr(test, ts(type = "number | null"))]
    pub universe_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct LocalProfileView {
    #[cfg_attr(test, ts(type = "number"))]
    pub user_id: u64,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    /// Only loaded for the selected profile.
    pub friend_count: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct PeopleOverview {
    #[cfg_attr(test, ts(type = "number | null"))]
    pub active_id: Option<u64>,
    pub profiles: Vec<LocalProfileView>,
    pub people: Vec<PersonView>,
    pub presence_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredUser {
    user_id: u64,
    username: String,
    display_name: String,
}

impl From<PublicUser> for StoredUser {
    fn from(user: PublicUser) -> Self {
        Self {
            user_id: user.id,
            username: bounded(&user.name),
            display_name: bounded(&user.display_name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredProfile {
    user: StoredUser,
    people: Vec<StoredUser>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PeopleDoc {
    active: Option<u64>,
    profiles: Vec<StoredProfile>,
}

impl PeopleDoc {
    fn active_profile(&self) -> Option<&StoredProfile> {
        let id = self.active?;
        self.profiles.iter().find(|profile| profile.user.user_id == id)
    }

    fn add_profile(&mut self, user: StoredUser) -> Result<(), RobloxError> {
        if self.profiles.iter().any(|profile| profile.user.user_id == user.user_id) {
            return Err(RobloxError::AlreadyAdded);
        }
        if self.profiles.len() >= MAX_PROFILES {
            return Err(RobloxError::ListFull);
        }
        self.active = Some(user.user_id);
        self.profiles.push(StoredProfile { user, people: Vec::new() });
        Ok(())
    }

    fn remove_profile(&mut self, id: u64) {
        self.profiles.retain(|profile| profile.user.user_id != id);
        if self.active == Some(id) {
            self.active = self.profiles.first().map(|profile| profile.user.user_id);
        }
    }

    fn set_active(&mut self, id: u64) -> Result<(), RobloxError> {
        if !self.profiles.iter().any(|profile| profile.user.user_id == id) {
            return Err(RobloxError::NoProfile);
        }
        self.active = Some(id);
        Ok(())
    }

    fn active_mut(&mut self) -> Result<&mut StoredProfile, RobloxError> {
        let id = self.active.ok_or(RobloxError::NoProfile)?;
        self.profiles
            .iter_mut()
            .find(|profile| profile.user.user_id == id)
            .ok_or(RobloxError::NoProfile)
    }

    fn add_person(&mut self, person: StoredUser) -> Result<(), RobloxError> {
        let profile = self.active_mut()?;
        if profile.user.user_id == person.user_id || profile.people.iter().any(|item| item.user_id == person.user_id) {
            return Err(RobloxError::AlreadyAdded);
        }
        if profile.people.len() >= MAX_PEOPLE {
            return Err(RobloxError::ListFull);
        }
        profile.people.push(person);
        Ok(())
    }

    fn remove_person(&mut self, id: u64) -> Result<(), RobloxError> {
        self.active_mut()?.people.retain(|person| person.user_id != id);
        Ok(())
    }
}

pub fn check_username(raw: &str) -> Result<String, RobloxError> {
    let name = raw.trim().trim_start_matches('@');
    let valid = (3..=20).contains(&name.len()) && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if valid {
        Ok(name.to_string())
    } else {
        Err(RobloxError::InvalidUsername)
    }
}

fn presence_state(code: Option<u8>) -> PresenceState {
    match code {
        Some(0) => PresenceState::Offline,
        Some(1) => PresenceState::Online,
        Some(2) => PresenceState::InGame,
        Some(3) => PresenceState::InStudio,
        Some(4) => PresenceState::Invisible,
        _ => PresenceState::Unknown,
    }
}

fn person_view(
    person: &StoredUser,
    names: &HashMap<u64, PublicUser>,
    avatars: &HashMap<u64, String>,
    presence: Option<&HashMap<u64, PresenceEntry>>,
) -> PersonView {
    let entry = presence.and_then(|items| items.get(&person.user_id));
    let state = presence.map_or(PresenceState::Unknown, |_| presence_state(entry.and_then(|item| item.user_presence_type)));
    let playing = matches!(state, PresenceState::InGame | PresenceState::InStudio);
    let current = names.get(&person.user_id);
    PersonView {
        user_id: person.user_id,
        username: current.map_or_else(|| person.username.clone(), |user| bounded(&user.name)),
        display_name: current.map_or_else(|| person.display_name.clone(), |user| bounded(&user.display_name)),
        avatar_url: avatars.get(&person.user_id).cloned(),
        presence: state,
        location: entry
            .filter(|_| playing)
            .and_then(|item| item.last_location.as_deref())
            .filter(|text| !text.trim().is_empty())
            .map(bounded),
        universe_id: entry.filter(|_| playing).and_then(|item| item.universe_id),
    }
}

fn build_overview(
    doc: &PeopleDoc,
    names: &HashMap<u64, PublicUser>,
    avatars: &HashMap<u64, String>,
    presence: Option<&HashMap<u64, PresenceEntry>>,
    friend_count: Option<f64>,
) -> PeopleOverview {
    let active_id = doc.active_profile().map(|profile| profile.user.user_id);
    let mut people: Vec<PersonView> = doc
        .active_profile()
        .map(|profile| {
            profile
                .people
                .iter()
                .map(|person| person_view(person, names, avatars, presence))
                .collect()
        })
        .unwrap_or_default();
    people.sort_by_key(|person| person.display_name.to_lowercase());

    PeopleOverview {
        active_id,
        profiles: doc
            .profiles
            .iter()
            .map(|profile| {
                let user = &profile.user;
                let current = names.get(&user.user_id);
                LocalProfileView {
                    user_id: user.user_id,
                    username: current.map_or_else(|| user.username.clone(), |item| bounded(&item.name)),
                    display_name: current.map_or_else(|| user.display_name.clone(), |item| bounded(&item.display_name)),
                    avatar_url: avatars.get(&user.user_id).cloned(),
                    friend_count: friend_count.filter(|_| active_id == Some(user.user_id)),
                }
            })
            .collect(),
        people,
        presence_available: presence.is_some(),
    }
}

pub struct PeopleManager {
    db: Arc<Database>,
    http: RobloxHttp,
    avatars: Mutex<HashMap<u64, (Instant, String)>>,
    counts: Mutex<HashMap<u64, (Instant, f64)>>,
}

impl PeopleManager {
    pub fn new(db: Arc<Database>) -> Result<Self, RobloxError> {
        Ok(Self {
            db,
            http: RobloxHttp::new()?,
            avatars: Mutex::new(HashMap::new()),
            counts: Mutex::new(HashMap::new()),
        })
    }

    fn read(&self) -> AppResult<PeopleDoc> {
        Ok(vault::read(&self.db.connection(), VAULT_KEY)?.unwrap_or_default())
    }

    fn update(&self, change: impl FnOnce(&mut PeopleDoc) -> Result<(), RobloxError>) -> AppResult<()> {
        let connection = self.db.connection();
        let mut doc: PeopleDoc = vault::read(&connection, VAULT_KEY)?.unwrap_or_default();
        change(&mut doc)?;
        vault::write(&connection, VAULT_KEY, &doc)
    }

    async fn load_avatars(&self, ids: &[u64]) -> HashMap<u64, String> {
        let mut found = HashMap::with_capacity(ids.len());
        let mut missing = Vec::new();
        {
            let cache = self.avatars.lock().unwrap_or_else(PoisonError::into_inner);
            for id in ids {
                match cache.get(id) {
                    Some((at, url)) if at.elapsed() < AVATAR_TTL => {
                        found.insert(*id, url.clone());
                    }
                    _ => missing.push(*id),
                }
            }
        }
        if missing.is_empty() {
            return found;
        }
        match people_api::headshots(&self.http, &missing).await {
            Ok(fresh) => {
                let now = Instant::now();
                let mut cache = self.avatars.lock().unwrap_or_else(PoisonError::into_inner);
                for (id, url) in &fresh {
                    cache.insert(*id, (now, url.clone()));
                }
                found.extend(fresh);
            }
            Err(error) => log::warn!("avatars unavailable: {error}"),
        }
        found
    }

    async fn load_friend_count(&self, id: Option<u64>) -> Option<f64> {
        let id = id?;
        {
            let cache = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((at, count)) = cache.get(&id) {
                if at.elapsed() < COUNT_TTL {
                    return Some(*count);
                }
            }
        }
        match people_api::friend_count(&self.http, id).await {
            Ok(count) => {
                self.counts
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(id, (Instant::now(), count));
                Some(count)
            }
            Err(error) => {
                log::warn!("friend count unavailable: {error}");
                None
            }
        }
    }

    /// Saved people are always returned, even when Roblox can't be reached.
    pub async fn overview(&self) -> AppResult<PeopleOverview> {
        let doc = self.read()?;
        let active = doc.active_profile();
        let person_ids: Vec<u64> = active
            .map(|profile| profile.people.iter().map(|person| person.user_id).collect())
            .unwrap_or_default();
        let active_id = active.map(|profile| profile.user.user_id);
        let mut all_ids: Vec<u64> = doc
            .profiles
            .iter()
            .map(|profile| profile.user.user_id)
            .chain(person_ids.iter().copied())
            .collect();
        all_ids.sort_unstable();
        all_ids.dedup();

        let (names, avatars, presence, friend_count) = tokio::join!(
            people_api::users(&self.http, &all_ids),
            self.load_avatars(&all_ids),
            people_api::presence(&self.http, &person_ids),
            self.load_friend_count(active_id),
        );
        let names = names.unwrap_or_else(|error| {
            log::warn!("profile names unavailable: {error}");
            HashMap::new()
        });
        let presence = match presence {
            Ok(items) => Some(items),
            Err(error) => {
                log::warn!("presence unavailable: {error}");
                None
            }
        };
        Ok(build_overview(&doc, &names, &avatars, presence.as_ref(), friend_count))
    }

    pub async fn add_profile(&self, username: &str) -> AppResult<PeopleOverview> {
        let name = check_username(username)?;
        let user = people_api::find_username(&self.http, &name).await?;
        self.update(|doc| doc.add_profile(user.into()))?;
        self.overview().await
    }

    pub async fn remove_profile(&self, id: u64) -> AppResult<PeopleOverview> {
        self.update(|doc| {
            doc.remove_profile(id);
            Ok(())
        })?;
        self.overview().await
    }

    pub async fn set_active(&self, id: u64) -> AppResult<PeopleOverview> {
        self.update(|doc| doc.set_active(id))?;
        self.overview().await
    }

    pub async fn add_person(&self, username: &str) -> AppResult<PeopleOverview> {
        let name = check_username(username)?;
        if self.read()?.active_profile().is_none() {
            return Err(RobloxError::NoProfile.into());
        }
        let user = people_api::find_username(&self.http, &name).await?;
        self.update(|doc| doc.add_person(user.into()))?;
        self.overview().await
    }

    pub async fn remove_person(&self, id: u64) -> AppResult<PeopleOverview> {
        self.update(|doc| doc.remove_person(id))?;
        self.overview().await
    }

    pub fn clear(&self) -> AppResult<()> {
        vault::remove(&self.db.connection(), VAULT_KEY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: u64, name: &str) -> StoredUser {
        StoredUser { user_id: id, username: name.into(), display_name: name.into() }
    }

    fn presence_of(id: u64, code: u8, location: &str, universe: Option<u64>) -> HashMap<u64, PresenceEntry> {
        HashMap::from([(
            id,
            PresenceEntry {
                user_id: id,
                user_presence_type: Some(code),
                last_location: Some(location.into()),
                universe_id: universe,
            },
        )])
    }

    #[test]
    fn usernames_are_validated() {
        assert_eq!(check_username("  @Player_1 ").unwrap(), "Player_1");
        for bad in ["", "ab", "has space", "bad-name", "toolongtoolongtoolong1", "名前前前"] {
            assert!(check_username(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn profiles_keep_separate_people() {
        let mut doc = PeopleDoc::default();
        doc.add_profile(user(1, "One")).unwrap();
        doc.add_person(user(10, "Ten")).unwrap();
        doc.add_profile(user(2, "Two")).unwrap();
        assert_eq!(doc.active, Some(2));
        assert!(doc.active_profile().unwrap().people.is_empty());
        doc.set_active(1).unwrap();
        assert_eq!(doc.active_profile().unwrap().people.len(), 1);
    }

    #[test]
    fn duplicates_and_self_are_refused() {
        let mut doc = PeopleDoc::default();
        doc.add_profile(user(1, "One")).unwrap();
        assert!(matches!(doc.add_profile(user(1, "One")), Err(RobloxError::AlreadyAdded)));
        assert!(matches!(doc.add_person(user(1, "One")), Err(RobloxError::AlreadyAdded)));
        doc.add_person(user(2, "Two")).unwrap();
        assert!(matches!(doc.add_person(user(2, "Two")), Err(RobloxError::AlreadyAdded)));
    }

    #[test]
    fn people_need_a_profile() {
        let mut doc = PeopleDoc::default();
        assert!(matches!(doc.add_person(user(2, "Two")), Err(RobloxError::NoProfile)));
    }

    #[test]
    fn list_limits_are_enforced() {
        let mut doc = PeopleDoc::default();
        for id in 1..=MAX_PROFILES as u64 {
            doc.add_profile(user(id, "P")).unwrap();
        }
        assert!(matches!(doc.add_profile(user(999, "P")), Err(RobloxError::ListFull)));
        for id in 0..MAX_PEOPLE as u64 {
            doc.add_person(user(1_000 + id, "F")).unwrap();
        }
        assert!(matches!(doc.add_person(user(5_000, "F")), Err(RobloxError::ListFull)));
    }

    #[test]
    fn removing_the_active_profile_selects_another() {
        let mut doc = PeopleDoc::default();
        doc.add_profile(user(1, "One")).unwrap();
        doc.add_profile(user(2, "Two")).unwrap();
        doc.remove_profile(2);
        assert_eq!(doc.active, Some(1));
        doc.remove_profile(1);
        assert_eq!(doc.active, None);
    }

    #[test]
    fn unknown_presence_is_never_reported_as_offline() {
        let person = user(5, "Five");
        let view = person_view(&person, &HashMap::new(), &HashMap::new(), None);
        assert_eq!(view.presence, PresenceState::Unknown);
    }

    #[test]
    fn location_is_only_kept_while_playing() {
        let person = user(5, "Five");
        let online = presence_of(5, 1, "Website", None);
        let view = person_view(&person, &HashMap::new(), &HashMap::new(), Some(&online));
        assert_eq!((view.presence, view.location), (PresenceState::Online, None));

        let playing = presence_of(5, 2, "A Game", Some(77));
        let view = person_view(&person, &HashMap::new(), &HashMap::new(), Some(&playing));
        assert_eq!(view.presence, PresenceState::InGame);
        assert_eq!(view.location.as_deref(), Some("A Game"));
        assert_eq!(view.universe_id, Some(77));
    }

    #[test]
    fn current_names_replace_saved_ones() {
        let person = user(5, "Old");
        let names = HashMap::from([(5, PublicUser { id: 5, name: "New".into(), display_name: "Shown".into() })]);
        let view = person_view(&person, &names, &HashMap::new(), None);
        assert_eq!((view.username.as_str(), view.display_name.as_str()), ("New", "Shown"));
    }

    #[test]
    fn friend_count_only_shows_on_the_active_profile() {
        let mut doc = PeopleDoc::default();
        doc.add_profile(user(1, "One")).unwrap();
        doc.add_profile(user(2, "Two")).unwrap();
        let overview = build_overview(&doc, &HashMap::new(), &HashMap::new(), None, Some(9.0));
        assert_eq!(overview.profiles[0].friend_count, None);
        assert_eq!(overview.profiles[1].friend_count, Some(9.0));
        assert!(!overview.presence_available);
    }

    #[test]
    fn saved_people_are_encrypted() {
        let db = Arc::new(Database::in_memory().unwrap());
        let manager = PeopleManager::new(Arc::clone(&db)).unwrap();
        manager.update(|doc| doc.add_profile(user(1, "PrivateName"))).unwrap();
        let raw: Vec<u8> = db
            .connection()
            .query_row("SELECT value FROM vault WHERE key = ?1", [VAULT_KEY], |row| row.get(0))
            .unwrap();
        assert!(!raw.windows(7).any(|window| window == b"Private"));
        manager.clear().unwrap();
        assert!(manager.read().unwrap().profiles.is_empty());
    }
}
