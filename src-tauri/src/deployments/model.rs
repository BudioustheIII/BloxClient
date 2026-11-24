use serde::{Deserialize, Serialize};

use super::error::DeploymentError;

/// A validated `version-<16 hex>` deployment id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionHash(String);

impl VersionHash {
    /// Accepts the id with or without its `version-` prefix, in any case.
    pub fn parse(input: &str) -> Result<Self, DeploymentError> {
        let normalized = input.trim().to_ascii_lowercase();
        let hex = normalized.strip_prefix("version-").unwrap_or(&normalized);
        if hex.len() == 16 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(format!("version-{hex}")))
        } else {
            Err(DeploymentError::Unavailable)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum DeploymentOrigin {
    /// Installed by BloxClient in its own folder. Removable.
    Managed,
    /// Installed by Roblox. BloxClient never changes these.
    Roblox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    Ready,
    Incomplete,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct InstalledDeployment {
    pub version: String,
    pub path: String,
    pub origin: DeploymentOrigin,
    pub status: DeploymentStatus,
    /// Unix seconds.
    pub installed_at: Option<f64>,
    pub size_bytes: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct DeploymentInfo {
    pub version: String,
    pub source_host: String,
    pub download_bytes: f64,
    pub install_bytes: f64,
    pub file_count: u32,
    pub installed: Vec<InstalledDeployment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "lowercase")]
pub enum InstallStage {
    Preparing,
    Downloading,
    Extracting,
    Verifying,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub version: String,
    pub stage: InstallStage,
    pub done_bytes: f64,
    pub total_bytes: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_versions() {
        for input in [
            "version-2c6fd5e09aaf4cfa",
            "  VERSION-2C6FD5E09AAF4CFA ",
            "2c6fd5e09aaf4cfa",
        ] {
            assert_eq!(
                VersionHash::parse(input).unwrap().as_str(),
                "version-2c6fd5e09aaf4cfa"
            );
        }
    }

    #[test]
    fn rejects_malformed_versions() {
        for input in [
            "",
            "version-",
            "version-2c6fd5e09aaf4cf",
            "version-2c6fd5e09aaf4cfaa",
            "version-2c6fd5e09aaf4cfg",
            "../../etc/passwd",
            "version-2c6fd5e0/aaf4cfa",
            "version-2c6fd5e09aaf4cfa&x=1",
        ] {
            assert!(VersionHash::parse(input).is_err(), "accepted {input:?}");
        }
    }
}
