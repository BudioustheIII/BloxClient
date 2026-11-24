//! Combines Roblox's list, detail and image APIs into UI models, with short
//! in-memory caches so moving between pages doesn't refetch everything.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use super::api::{self, ExploreSort, GameDetails, ListedGame};
use super::error::RobloxError;
use super::http::RobloxHttp;
use super::model::{DiscoverFeed, ExperienceCard, ExperienceDetail, ExperienceRow, LaunchTarget, SearchPage};

const MAX_ROWS: usize = 8;
const MAX_PER_ROW: usize = 24;
const DETAIL_IMAGES: u8 = 10;
const MAX_QUERY_CHARS: usize = 100;
const FEED_TTL: Duration = Duration::from_secs(5 * 60);
const META_TTL: Duration = Duration::from_secs(30 * 60);
const DETAIL_TTL: Duration = Duration::from_secs(60);
const MAX_CACHED: usize = 2_000;

#[derive(Debug, Clone, Default)]
struct CardMeta {
    creator_name: Option<String>,
    creator_verified: bool,
    thumbnail_url: Option<String>,
}

pub struct Catalog {
    http: RobloxHttp,
    feed: Mutex<Option<(Instant, DiscoverFeed)>>,
    meta: Mutex<HashMap<u64, (Instant, CardMeta)>>,
    details: Mutex<HashMap<u64, (Instant, ExperienceDetail)>>,
}

/// Keeps game rows, in order, without ads or repeated entries.
fn build_rows(sorts: Vec<ExploreSort>) -> Vec<(String, String, Vec<ListedGame>)> {
    let mut seen_rows = HashSet::new();
    sorts
        .into_iter()
        .filter(|sort| sort.content_type.as_deref() == Some("Games"))
        .filter_map(|sort| {
            let id = sort.sort_id?;
            let title = sort.sort_display_name.filter(|name| !name.trim().is_empty())?;
            if !seen_rows.insert(id.clone()) {
                return None;
            }
            let games = clean_games(sort.games, MAX_PER_ROW);
            (!games.is_empty()).then_some((id, title, games))
        })
        .take(MAX_ROWS)
        .collect()
}

fn clean_games(games: Vec<ListedGame>, limit: usize) -> Vec<ListedGame> {
    let mut seen = HashSet::new();
    games
        .into_iter()
        .filter(|game| !game.is_sponsored && game.root_place_id != 0 && seen.insert(game.universe_id))
        .take(limit)
        .collect()
}

fn to_card(game: ListedGame, meta: Option<&CardMeta>) -> ExperienceCard {
    ExperienceCard {
        universe_id: game.universe_id,
        place_id: game.root_place_id,
        name: game.name,
        creator_name: meta.and_then(|meta| meta.creator_name.clone()),
        creator_verified: meta.is_some_and(|meta| meta.creator_verified),
        playing: game.player_count,
        up_votes: game.total_up_votes,
        down_votes: game.total_down_votes,
        thumbnail_url: meta.and_then(|meta| meta.thumbnail_url.clone()),
        genre: game.genre.filter(|genre| !genre.is_empty()),
        maturity: game.content_maturity.filter(|value| !value.is_empty()),
    }
}

fn evict<V>(cache: &mut HashMap<u64, (Instant, V)>, ttl: Duration) {
    if cache.len() >= MAX_CACHED {
        cache.retain(|_, (at, _)| at.elapsed() < ttl);
        if cache.len() >= MAX_CACHED {
            cache.clear();
        }
    }
}

impl Catalog {
    pub fn new() -> Result<Self, RobloxError> {
        Ok(Self {
            http: RobloxHttp::new()?,
            feed: Mutex::new(None),
            meta: Mutex::new(HashMap::new()),
            details: Mutex::new(HashMap::new()),
        })
    }

    pub async fn feed(&self, refresh: bool) -> Result<DiscoverFeed, RobloxError> {
        if !refresh {
            let cached = self.feed.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((at, feed)) = &*cached {
                if at.elapsed() < FEED_TTL {
                    return Ok(feed.clone());
                }
            }
        }

        let first = api::explore(&self.http, None).await?;
        let mut sorts = first.sorts;
        if let Some(token) = first.next_sorts_page_token.as_deref() {
            // A second page only adds variety, so failing here isn't an error.
            match api::explore(&self.http, Some(token)).await {
                Ok(next) => sorts.extend(next.sorts),
                Err(error) => log::warn!("could not load more discovery rows: {error}"),
            }
        }

        let rows = build_rows(sorts);
        if rows.is_empty() {
            return Err(RobloxError::UnexpectedResponse("no game rows in explore response".into()));
        }

        let ids: Vec<u64> = rows
            .iter()
            .flat_map(|(_, _, games)| games.iter().map(|game| game.universe_id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let meta = self.hydrate(&ids).await;

        let feed = DiscoverFeed {
            rows: rows
                .into_iter()
                .map(|(id, title, games)| ExperienceRow {
                    id,
                    title,
                    experiences: games
                        .into_iter()
                        .map(|game| {
                            let entry = meta.get(&game.universe_id);
                            to_card(game, entry)
                        })
                        .collect(),
                })
                .collect(),
        };
        *self.feed.lock().unwrap_or_else(PoisonError::into_inner) = Some((Instant::now(), feed.clone()));
        Ok(feed)
    }

    pub async fn search(&self, query: &str, page_token: Option<&str>) -> Result<SearchPage, RobloxError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(RobloxError::InvalidSearch("enter a game name"));
        }
        if query.chars().count() > MAX_QUERY_CHARS {
            return Err(RobloxError::InvalidSearch("searches are limited to 100 characters"));
        }

        let (games, next_page) = api::search(&self.http, query, page_token).await?;
        let games = clean_games(games, usize::MAX);
        let ids: Vec<u64> = games.iter().map(|game| game.universe_id).collect();
        let meta = self.hydrate(&ids).await;
        Ok(SearchPage {
            results: games
                .into_iter()
                .map(|game| {
                    let entry = meta.get(&game.universe_id);
                    to_card(game, entry)
                })
                .collect(),
            next_page: next_page.filter(|token| !token.is_empty()),
        })
    }

    pub async fn detail(&self, universe_id: u64) -> Result<ExperienceDetail, RobloxError> {
        {
            let cached = self.details.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((at, detail)) = cached.get(&universe_id) {
                if at.elapsed() < DETAIL_TTL {
                    return Ok(detail.clone());
                }
            }
        }

        let ids = [universe_id];
        let (games, votes, images, icon) = tokio::join!(
            api::game_details(&self.http, &ids),
            api::votes(&self.http, universe_id),
            api::wide_images(&self.http, &ids, DETAIL_IMAGES),
            api::icon(&self.http, universe_id),
        );
        let game = games?.into_iter().find(|game| game.id == universe_id).ok_or(RobloxError::NotFound)?;

        // Votes and images are extras: the page is still useful without them.
        let votes = votes.unwrap_or_else(|error| {
            log::warn!("votes unavailable for {universe_id}: {error}");
            None
        });
        let images = images
            .unwrap_or_else(|error| {
                log::warn!("images unavailable for {universe_id}: {error}");
                HashMap::new()
            })
            .remove(&universe_id)
            .unwrap_or_default();
        let icon_url = icon.unwrap_or_else(|error| {
            log::warn!("icon unavailable for {universe_id}: {error}");
            None
        });

        let detail = to_detail(game, votes, images, icon_url);
        let mut cache = self.details.lock().unwrap_or_else(PoisonError::into_inner);
        evict(&mut cache, DETAIL_TTL);
        cache.insert(universe_id, (Instant::now(), detail.clone()));
        Ok(detail)
    }

    /// Names and images for games in play history. Uses each game's start
    /// place, since sub-places often can't be joined directly. Games that
    /// can't be loaded are left out and retried on the next sync.
    pub async fn launch_targets(&self, ids: &[u64]) -> HashMap<u64, LaunchTarget> {
        let (games, images) = tokio::join!(
            api::game_details(&self.http, ids),
            api::wide_images(&self.http, ids, 1),
        );
        let games = match games {
            Ok(games) => games,
            Err(error) => {
                log::warn!("played game details unavailable: {error}");
                return HashMap::new();
            }
        };
        let mut images = images.unwrap_or_else(|error| {
            log::warn!("played game images unavailable: {error}");
            HashMap::new()
        });
        games
            .into_iter()
            .map(|game| {
                let thumbnail_url = images.remove(&game.id).and_then(|urls| urls.into_iter().next());
                let target = LaunchTarget {
                    universe_id: game.id,
                    place_id: game.root_place_id,
                    name: game.name,
                    creator_name: game.creator.and_then(|creator| creator.name),
                    thumbnail_url,
                };
                (game.id, target)
            })
            .collect()
    }

    /// Loads creators and images. Failures are logged and leave those fields
    /// empty, because a card without an image is better than no results.
    async fn hydrate(&self, ids: &[u64]) -> HashMap<u64, CardMeta> {
        let mut found = HashMap::with_capacity(ids.len());
        let mut missing = Vec::new();
        {
            let cache = self.meta.lock().unwrap_or_else(PoisonError::into_inner);
            for id in ids {
                match cache.get(id) {
                    Some((at, meta)) if at.elapsed() < META_TTL => {
                        found.insert(*id, meta.clone());
                    }
                    _ => missing.push(*id),
                }
            }
        }
        if missing.is_empty() {
            return found;
        }

        let (games, images) = tokio::join!(
            api::game_details(&self.http, &missing),
            api::wide_images(&self.http, &missing, 1),
        );
        let games_loaded = games.is_ok();
        let images_loaded = images.is_ok();
        let games = games.unwrap_or_else(|error| {
            log::warn!("creator details unavailable: {error}");
            Vec::new()
        });
        let mut images = images.unwrap_or_else(|error| {
            log::warn!("thumbnails unavailable: {error}");
            HashMap::new()
        });

        let mut fresh: HashMap<u64, CardMeta> = games
            .into_iter()
            .map(|game| {
                let creator = game.creator.unwrap_or(api::Creator {
                    name: None,
                    has_verified_badge: false,
                });
                (
                    game.id,
                    CardMeta {
                        creator_name: creator.name,
                        creator_verified: creator.has_verified_badge,
                        thumbnail_url: None,
                    },
                )
            })
            .collect();
        for id in &missing {
            if let Some(url) = images.remove(id).and_then(|urls| urls.into_iter().next()) {
                fresh.entry(*id).or_default().thumbnail_url = Some(url);
            }
        }

        // Partial results are shown but not cached, so they can fill in later.
        if games_loaded && images_loaded {
            let mut cache = self.meta.lock().unwrap_or_else(PoisonError::into_inner);
            evict(&mut cache, META_TTL);
            let now = Instant::now();
            for (id, meta) in &fresh {
                cache.insert(*id, (now, meta.clone()));
            }
        }
        found.extend(fresh);
        found
    }
}

fn to_detail(
    game: GameDetails,
    votes: Option<api::Votes>,
    images: Vec<String>,
    icon_url: Option<String>,
) -> ExperienceDetail {
    let creator = game.creator.clone();
    ExperienceDetail {
        card: ExperienceCard {
            universe_id: game.id,
            place_id: game.root_place_id,
            name: game.name,
            creator_name: creator.as_ref().and_then(|creator| creator.name.clone()),
            creator_verified: creator.is_some_and(|creator| creator.has_verified_badge),
            playing: game.playing,
            up_votes: votes.map(|votes| votes.up_votes),
            down_votes: votes.map(|votes| votes.down_votes),
            thumbnail_url: images.first().cloned(),
            genre: game.genre_l1.or(game.genre).filter(|genre| !genre.is_empty()),
            maturity: None,
        },
        description: game.description.filter(|text| !text.trim().is_empty()),
        visits: game.visits,
        favorites: game.favorited_count,
        max_players: game.max_players,
        created: game.created,
        updated: game.updated,
        icon_url,
        images,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(id: u64, sponsored: bool) -> serde_json::Value {
        json!({ "universeId": id, "rootPlaceId": id * 10, "name": format!("Game {id}"), "isSponsored": sponsored })
    }

    fn sort(id: &str, title: &str, kind: &str, games: Vec<serde_json::Value>) -> ExploreSort {
        serde_json::from_value(json!({
            "contentType": kind, "sortId": id, "sortDisplayName": title, "games": games
        }))
        .unwrap()
    }

    #[test]
    fn rows_exclude_ads_duplicates_and_non_game_sections() {
        let rows = build_rows(vec![
            sort("filters", "Filters", "Filters", vec![game(9, false)]),
            sort("top", "Top", "Games", vec![game(1, false), game(2, true), game(1, false), game(3, false)]),
            sort("top", "Repeat", "Games", vec![game(4, false)]),
            sort("empty", "Only ads", "Games", vec![game(5, true)]),
            sort("untitled", "  ", "Games", vec![game(6, false)]),
        ]);
        assert_eq!(rows.len(), 1);
        let ids: Vec<u64> = rows[0].2.iter().map(|game| game.universe_id).collect();
        assert_eq!(ids, [1, 3]);
    }

    #[test]
    fn rows_and_row_lengths_are_capped() {
        let games: Vec<_> = (1..=40).map(|id| game(id, false)).collect();
        let sorts = (0..12).map(|index| sort(&format!("s{index}"), "Row", "Games", games.clone())).collect();
        let rows = build_rows(sorts);
        assert_eq!(rows.len(), MAX_ROWS);
        assert_eq!(rows[0].2.len(), MAX_PER_ROW);
    }

    #[test]
    fn card_without_metadata_has_no_invented_values() {
        let listed: ListedGame = serde_json::from_value(game(1, false)).unwrap();
        let card = to_card(listed, None);
        assert_eq!(card.creator_name, None);
        assert_eq!(card.thumbnail_url, None);
        assert_eq!(card.playing, None);
        assert!(!card.creator_verified);
    }

    #[test]
    fn empty_description_becomes_none() {
        let details: GameDetails = serde_json::from_value(json!({
            "id": 1, "rootPlaceId": 2, "name": "G", "description": "   ", "genre_l1": "Action"
        }))
        .unwrap();
        let detail = to_detail(details, None, vec![], None);
        assert_eq!(detail.description, None);
        assert_eq!(detail.card.genre.as_deref(), Some("Action"));
        assert_eq!(detail.card.up_votes, None);
    }
}
