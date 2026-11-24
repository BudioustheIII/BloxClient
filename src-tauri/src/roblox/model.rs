//! Types shared with the frontend. Roblox ids stay below 2^53, so they are
//! safe as JavaScript numbers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct ExperienceCard {
    #[cfg_attr(test, ts(type = "number"))]
    pub universe_id: u64,
    #[cfg_attr(test, ts(type = "number"))]
    pub place_id: u64,
    pub name: String,
    pub creator_name: Option<String>,
    pub creator_verified: bool,
    pub playing: Option<f64>,
    pub up_votes: Option<f64>,
    pub down_votes: Option<f64>,
    pub thumbnail_url: Option<String>,
    pub genre: Option<String>,
    pub maturity: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct ExperienceRow {
    pub id: String,
    pub title: String,
    pub experiences: Vec<ExperienceCard>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct DiscoverFeed {
    pub rows: Vec<ExperienceRow>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub results: Vec<ExperienceCard>,
    pub next_page: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct ExperienceDetail {
    pub card: ExperienceCard,
    pub description: Option<String>,
    pub visits: Option<f64>,
    pub favorites: Option<f64>,
    pub max_players: Option<f64>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub icon_url: Option<String>,
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct RecentExperience {
    #[cfg_attr(test, ts(type = "number"))]
    pub universe_id: u64,
    #[cfg_attr(test, ts(type = "number"))]
    pub place_id: u64,
    pub name: String,
    pub creator_name: Option<String>,
    pub thumbnail_url: Option<String>,
    pub last_played_at: f64,
    pub play_count: f64,
    pub total_seconds: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct LaunchTarget {
    #[cfg_attr(test, ts(type = "number"))]
    pub universe_id: u64,
    #[cfg_attr(test, ts(type = "number"))]
    pub place_id: u64,
    pub name: String,
    pub creator_name: Option<String>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum SessionStage {
    Launching,
    Running,
    Ended,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct PlaySession {
    #[cfg_attr(test, ts(type = "number"))]
    pub universe_id: u64,
    #[cfg_attr(test, ts(type = "number"))]
    pub place_id: u64,
    pub name: String,
    pub thumbnail_url: Option<String>,
    pub stage: SessionStage,
    pub running_since: Option<f64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct RobloxInstall {
    /// Display name of the program that opens `roblox://` links.
    pub launcher_name: Option<String>,
    pub launcher_path: Option<String>,
    pub launcher_found: bool,
    pub is_official_launcher: bool,
    pub player_running: bool,
}
