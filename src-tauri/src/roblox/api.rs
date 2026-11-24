//! Adapters for Roblox's public endpoints.
//!
//! `games` and `thumbnails` are stable public APIs. The explore and search
//! endpoints are what roblox.com itself uses and are undocumented, so every
//! field is optional and one malformed item never fails a whole response.

use std::collections::HashMap;

use reqwest::Url;
use serde::de::{DeserializeOwned, Deserializer};
use serde::Deserialize;
use serde_json::Value;

use super::error::RobloxError;
use super::http::RobloxHttp;

const GAMES: &str = "https://games.roblox.com/v1/games";
const THUMBNAILS: &str = "https://thumbnails.roblox.com/v1/games";
const EXPLORE: &str = "https://apis.roblox.com/explore-api/v1/get-sorts";
const SEARCH: &str = "https://apis.roblox.com/search-api/omni-search";
const BATCH_SIZE: usize = 50;
const WIDE_SIZE: &str = "768x432";
const ICON_SIZE: &str = "256x256";

pub(super) fn lenient<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let items: Option<Vec<Value>> = Option::deserialize(deserializer)?;
    Ok(items
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| serde_json::from_value(item).ok())
        .collect())
}

/// A game entry from the explore or search APIs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedGame {
    pub universe_id: u64,
    pub root_place_id: u64,
    pub name: String,
    pub player_count: Option<f64>,
    pub total_up_votes: Option<f64>,
    pub total_down_votes: Option<f64>,
    #[serde(default)]
    pub is_sponsored: bool,
    pub content_maturity: Option<String>,
    #[serde(rename = "genreL1")]
    pub genre: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreSort {
    pub content_type: Option<String>,
    pub sort_id: Option<String>,
    pub sort_display_name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub games: Vec<ListedGame>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResponse {
    #[serde(default, deserialize_with = "lenient")]
    pub sorts: Vec<ExploreSort>,
    pub next_sorts_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchGroup {
    #[serde(rename = "contentGroupType")]
    group_type: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    contents: Vec<ListedGame>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    #[serde(default, deserialize_with = "lenient")]
    search_results: Vec<SearchGroup>,
    next_page_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub name: Option<String>,
    #[serde(default)]
    pub has_verified_badge: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetails {
    pub id: u64,
    pub root_place_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub creator: Option<Creator>,
    pub playing: Option<f64>,
    pub visits: Option<f64>,
    pub max_players: Option<f64>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub favorited_count: Option<f64>,
    #[serde(rename = "genre_l1")]
    pub genre_l1: Option<String>,
    pub genre: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DataList<T> {
    #[serde(default = "Vec::new", bound(deserialize = "T: DeserializeOwned"), deserialize_with = "lenient")]
    data: Vec<T>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Votes {
    pub id: u64,
    pub up_votes: f64,
    pub down_votes: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Image {
    state: Option<String>,
    image_url: Option<String>,
}

impl Image {
    /// Only finished images on Roblox's image CDN are passed to the UI.
    fn usable(self) -> Option<String> {
        let url = self.image_url?;
        (self.state.as_deref() == Some("Completed") && url.starts_with("https://tr.rbxcdn.com/")).then_some(url)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UniverseImages {
    universe_id: u64,
    #[serde(default, deserialize_with = "lenient")]
    thumbnails: Vec<Image>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IconEntry {
    target_id: u64,
    state: Option<String>,
    image_url: Option<String>,
}

fn url(base: &str, params: &[(&str, &str)]) -> Result<Url, RobloxError> {
    Url::parse_with_params(base, params).map_err(|error| RobloxError::UnexpectedResponse(error.to_string()))
}

fn join_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

pub async fn explore(http: &RobloxHttp, page_token: Option<&str>) -> Result<ExploreResponse, RobloxError> {
    let mut params = vec![("sessionId", http.session_id())];
    if let Some(token) = page_token {
        params.push(("sortsPageToken", token));
    }
    http.get_json(url(EXPLORE, &params)?).await
}

pub async fn search(
    http: &RobloxHttp,
    query: &str,
    page_token: Option<&str>,
) -> Result<(Vec<ListedGame>, Option<String>), RobloxError> {
    let mut params = vec![
        ("searchQuery", query),
        ("sessionId", http.session_id()),
        ("pageType", "all"),
    ];
    if let Some(token) = page_token {
        params.push(("pageToken", token));
    }
    let response: SearchResponse = http.get_json(url(SEARCH, &params)?).await?;
    let games = response
        .search_results
        .into_iter()
        .filter(|group| group.group_type.as_deref() == Some("Game"))
        .flat_map(|group| group.contents)
        .collect();
    Ok((games, response.next_page_token))
}

pub async fn game_details(http: &RobloxHttp, ids: &[u64]) -> Result<Vec<GameDetails>, RobloxError> {
    let mut games = Vec::with_capacity(ids.len());
    for batch in ids.chunks(BATCH_SIZE) {
        let joined = join_ids(batch);
        let page: DataList<GameDetails> = http.get_json(url(GAMES, &[("universeIds", &joined)])?).await?;
        games.extend(page.data);
    }
    Ok(games)
}

pub async fn votes(http: &RobloxHttp, id: u64) -> Result<Option<Votes>, RobloxError> {
    let joined = id.to_string();
    let endpoint = format!("{GAMES}/votes");
    let page: DataList<Votes> = http.get_json(url(&endpoint, &[("universeIds", &joined)])?).await?;
    Ok(page.data.into_iter().find(|entry| entry.id == id))
}

/// Widescreen images per universe, in Roblox's display order.
pub async fn wide_images(http: &RobloxHttp, ids: &[u64], per_universe: u8) -> Result<HashMap<u64, Vec<String>>, RobloxError> {
    let endpoint = format!("{THUMBNAILS}/multiget/thumbnails");
    let count = per_universe.to_string();
    let mut images = HashMap::with_capacity(ids.len());
    for batch in ids.chunks(BATCH_SIZE) {
        let joined = join_ids(batch);
        let params = [
            ("universeIds", joined.as_str()),
            ("countPerUniverse", count.as_str()),
            ("size", WIDE_SIZE),
            ("format", "Webp"),
            ("isCircular", "false"),
        ];
        let page: DataList<UniverseImages> = http.get_json(url(&endpoint, &params)?).await?;
        for entry in page.data {
            let urls = entry.thumbnails.into_iter().filter_map(Image::usable).collect();
            images.insert(entry.universe_id, urls);
        }
    }
    Ok(images)
}

pub async fn icon(http: &RobloxHttp, id: u64) -> Result<Option<String>, RobloxError> {
    let endpoint = format!("{THUMBNAILS}/icons");
    let joined = id.to_string();
    let params = [
        ("universeIds", joined.as_str()),
        ("size", ICON_SIZE),
        ("format", "Webp"),
        ("isCircular", "false"),
    ];
    let page: DataList<IconEntry> = http.get_json(url(&endpoint, &params)?).await?;
    Ok(page.data.into_iter().find(|entry| entry.target_id == id).and_then(|entry| {
        Image {
            state: entry.state,
            image_url: entry.image_url,
        }
        .usable()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explore_tolerates_unknown_shapes() {
        let response: ExploreResponse = serde_json::from_value(json!({
            "sorts": [
                { "contentType": "Filters", "filters": [] },
                { "contentType": "Games", "sortId": "top", "sortDisplayName": "Top", "games": [
                    { "universeId": 1, "rootPlaceId": 2, "name": "Good", "playerCount": 5 },
                    { "universeId": "broken", "name": 7 },
                    { "universeId": 3, "rootPlaceId": 4, "name": "Ad", "isSponsored": true, "futureField": {} }
                ]},
                "not even an object"
            ],
            "nextSortsPageToken": null
        }))
        .unwrap();
        assert_eq!(response.sorts.len(), 2);
        assert_eq!(response.sorts[1].games.len(), 2);
        assert!(response.sorts[1].games[1].is_sponsored);
    }

    #[test]
    fn only_completed_rbxcdn_images_are_usable() {
        let image = |state: &str, url: &str| Image {
            state: Some(state.into()),
            image_url: Some(url.into()),
        };
        assert!(image("Completed", "https://tr.rbxcdn.com/a/768/432/Image/Webp/noFilter").usable().is_some());
        assert!(image("Pending", "https://tr.rbxcdn.com/a").usable().is_none());
        assert!(image("Completed", "https://evil.example/a.png").usable().is_none());
        assert!(image("Completed", "https://tr.rbxcdn.com.evil.example/a").usable().is_none());
    }

    #[test]
    fn query_values_are_encoded() {
        let built = url(SEARCH, &[("searchQuery", "a&b=c d")]).unwrap();
        assert_eq!(built.query(), Some("searchQuery=a%26b%3Dc+d"));
    }
}
