use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum FlagKind {
    String,
    Boolean,
    Number,
}

/// A flag as saved. `value` is always in canonical form for its kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagEntry {
    pub name: String,
    pub kind: FlagKind,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct FlagInput {
    pub name: String,
    pub kind: FlagKind,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct FlagView {
    pub name: String,
    pub kind: FlagKind,
    pub value: String,
    pub enabled: bool,
    /// On the baseline list Roblox published. Not proof the flag still works.
    pub listed: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum ProfileSource {
    /// Recommended by BloxClient. Read-only.
    Preset,
    /// Created by the user.
    User,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub id: String,
    pub name: String,
    pub source: ProfileSource,
    pub flags: Vec<FlagView>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    #[cfg_attr(test, ts(type = "number"))]
    pub id: i64,
    /// Unix seconds.
    pub created_at: f64,
    pub reason: String,
    pub profile_count: u32,
    pub flag_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct FlagOverview {
    pub active_id: String,
    pub profiles: Vec<ProfileView>,
    pub backups: Vec<BackupInfo>,
}
