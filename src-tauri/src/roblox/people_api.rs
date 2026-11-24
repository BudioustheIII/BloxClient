//! Public, signed-out Roblox user endpoints. Checked on 2026-11-23:
//! usernames, profiles, headshots, presence and friend counts answer without
//! a login, but the friends list itself returns 403.

use std::collections::HashMap;

use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::api::lenient;
use super::error::RobloxError;
use super::http::RobloxHttp;

const USERNAME_LOOKUP: &str = "https://users.roblox.com/v1/usernames/users";
const USERS: &str = "https://users.roblox.com/v1/users";
const PRESENCE: &str = "https://presence.roblox.com/v1/presence/users";
const HEADSHOTS: &str = "https://thumbnails.roblox.com/v1/users/avatar-headshot";
const FRIENDS: &str = "https://friends.roblox.com/v1/users";
const USER_BATCH: usize = 100;
const PRESENCE_BATCH: usize = 50;

fn url(text: &str) -> Result<Url, RobloxError> {
    Url::parse(text).map_err(|error| RobloxError::UnexpectedResponse(error.to_string()))
}

#[derive(Debug, Deserialize)]
struct Data<T> {
    #[serde(default = "Vec::new", bound(deserialize = "T: DeserializeOwned"), deserialize_with = "lenient")]
    data: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    pub id: u64,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceEntry {
    pub user_id: u64,
    pub user_presence_type: Option<u8>,
    pub last_location: Option<String>,
    pub universe_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresenceResponse {
    #[serde(default, deserialize_with = "lenient")]
    user_presences: Vec<PresenceEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Headshot {
    target_id: u64,
    state: Option<String>,
    image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Count {
    count: f64,
}

pub async fn find_username(http: &RobloxHttp, username: &str) -> Result<PublicUser, RobloxError> {
    let body = json!({ "usernames": [username], "excludeBannedUsers": true });
    let found: Data<PublicUser> = http.post_json(url(USERNAME_LOOKUP)?, &body).await?;
    found.data.into_iter().next().ok_or(RobloxError::UserNotFound)
}

pub async fn users(http: &RobloxHttp, ids: &[u64]) -> Result<HashMap<u64, PublicUser>, RobloxError> {
    let mut all = HashMap::with_capacity(ids.len());
    for batch in ids.chunks(USER_BATCH) {
        let body = json!({ "userIds": batch, "excludeBannedUsers": false });
        let found: Data<PublicUser> = http.post_json(url(USERS)?, &body).await?;
        all.extend(found.data.into_iter().map(|user| (user.id, user)));
    }
    Ok(all)
}

pub async fn presence(http: &RobloxHttp, ids: &[u64]) -> Result<HashMap<u64, PresenceEntry>, RobloxError> {
    let mut all = HashMap::with_capacity(ids.len());
    for batch in ids.chunks(PRESENCE_BATCH) {
        let found: PresenceResponse = http.post_json(url(PRESENCE)?, &json!({ "userIds": batch })).await?;
        all.extend(found.user_presences.into_iter().map(|entry| (entry.user_id, entry)));
    }
    Ok(all)
}

pub async fn headshots(http: &RobloxHttp, ids: &[u64]) -> Result<HashMap<u64, String>, RobloxError> {
    let mut all = HashMap::with_capacity(ids.len());
    for batch in ids.chunks(USER_BATCH) {
        let joined = batch.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let address = Url::parse_with_params(
            HEADSHOTS,
            [("userIds", joined.as_str()), ("size", "150x150"), ("format", "Png"), ("isCircular", "false")],
        )
        .map_err(|error| RobloxError::UnexpectedResponse(error.to_string()))?;
        let found: Data<Headshot> = http.get_json(address).await?;
        all.extend(found.data.into_iter().filter_map(|item| {
            let ready = item.state.as_deref() == Some("Completed");
            let image = super::history::trusted_image(item.image_url.as_deref())?;
            ready.then_some((item.target_id, image))
        }));
    }
    Ok(all)
}

pub async fn friend_count(http: &RobloxHttp, id: u64) -> Result<f64, RobloxError> {
    let found: Count = http.get_json(url(&format!("{FRIENDS}/{id}/friends/count"))?).await?;
    Ok(found.count)
}
