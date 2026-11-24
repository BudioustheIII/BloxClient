//! Settings schema. TypeScript bindings are generated from these types by
//! `cargo test`, so the frontend cannot drift from the backend.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const MIN_UI_SCALE: u16 = 80;
pub const MAX_UI_SCALE: u16 = 150;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub roblox: RobloxSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase", default)]
pub struct RobloxSettings {
    pub deployment_host: DeploymentHost,
}

/// Roblox's official download hosts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum DeploymentHost {
    #[default]
    Auto,
    Primary,
    Aws,
    Akamai,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub launch_on_startup: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub default_page: PageId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase", default)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub accent: String,
    pub sidebar: SidebarMode,
    pub motion: MotionMode,
    pub ui_scale: u16,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            accent: "#E8E8E6".into(),
            sidebar: SidebarMode::default(),
            motion: MotionMode::default(),
            ui_scale: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum PageId {
    #[default]
    Home,
    Discover,
    Library,
    Favorites,
    Friends,
    Performance,
    Accounts,
    Settings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum SidebarMode {
    #[default]
    Expanded,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum MotionMode {
    #[default]
    Full,
    Reduced,
    Off,
}

impl Settings {
    pub fn validate(&self) -> AppResult<()> {
        let accent = self.appearance.accent.as_bytes();
        let valid_accent = accent.len() == 7
            && accent[0] == b'#'
            && accent[1..].iter().all(u8::is_ascii_hexdigit);
        if !valid_accent {
            return Err(AppError::InvalidSettings(
                "accent color must look like #E8E8E6".into(),
            ));
        }

        let scale = self.appearance.ui_scale;
        if !(MIN_UI_SCALE..=MAX_UI_SCALE).contains(&scale) {
            return Err(AppError::InvalidSettings(format!(
                "interface scale must be between {MIN_UI_SCALE}% and {MAX_UI_SCALE}%"
            )));
        }
        Ok(())
    }
}
