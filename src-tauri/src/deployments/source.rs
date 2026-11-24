//! HTTP access to Roblox's official deployment hosts. Only these fixed hosts
//! are contacted, so a version id can never redirect a download elsewhere.

use std::time::Duration;

use reqwest::{Client, StatusCode};

use super::error::DeploymentError;
use super::manifest::{parse_packages, FileManifest, Package};
use super::model::VersionHash;
use crate::settings::DeploymentHost;

const PRIMARY: &str = "https://setup.rbxcdn.com";
const AWS: &str = "https://setup-aws.rbxcdn.com";
const AKAMAI: &str = "https://setup-ak.rbxcdn.com";
const MAX_MANIFEST_BYTES: usize = 8 * 1024 * 1024;

pub fn hosts_for(choice: DeploymentHost) -> &'static [&'static str] {
    match choice {
        DeploymentHost::Auto => &[PRIMARY, AWS, AKAMAI],
        DeploymentHost::Primary => &[PRIMARY],
        DeploymentHost::Aws => &[AWS],
        DeploymentHost::Akamai => &[AKAMAI],
    }
}

pub fn build_client() -> Result<Client, DeploymentError> {
    Client::builder()
        .user_agent(concat!("BloxClient/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(network)
}

pub fn network(error: impl std::fmt::Display) -> DeploymentError {
    DeploymentError::Network(error.to_string())
}

pub fn package_url(host: &str, version: &VersionHash, name: &str) -> String {
    format!("{host}/{}-{name}", version.as_str())
}

pub struct Manifests {
    pub host: &'static str,
    pub packages: Vec<Package>,
    pub files: FileManifest,
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, DeploymentError> {
    let mut response = client.get(url).send().await.map_err(network)?;
    match response.status() {
        StatusCode::OK => {}
        // The CDN answers 403 for objects that don't exist.
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => return Err(DeploymentError::Unavailable),
        status => return Err(network(format!("HTTP {status}"))),
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if body.len() + chunk.len() > MAX_MANIFEST_BYTES {
            return Err(DeploymentError::Manifest("response too large".into()));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| DeploymentError::Manifest("not text".into()))
}

/// A version counts as available only if a host serves both manifests and
/// both parse. Status codes alone are never treated as proof.
pub async fn fetch_manifests(
    client: &Client,
    version: &VersionHash,
    choice: DeploymentHost,
) -> Result<Manifests, DeploymentError> {
    let mut last_error = DeploymentError::Unavailable;
    for &host in hosts_for(choice) {
        match fetch_from(client, host, version).await {
            Ok(manifests) => return Ok(manifests),
            Err(error) => {
                log::warn!("{host} could not serve {}: {error}", version.as_str());
                // Prefer reporting a connection problem over "doesn't exist".
                if !matches!(last_error, DeploymentError::Network(_)) {
                    last_error = error;
                }
            }
        }
    }
    Err(last_error)
}

async fn fetch_from(client: &Client, host: &'static str, version: &VersionHash) -> Result<Manifests, DeploymentError> {
    let packages = fetch_text(client, &package_url(host, version, "rbxPkgManifest.txt")).await?;
    let files = fetch_text(client, &package_url(host, version, "rbxManifest.txt")).await?;
    Ok(Manifests {
        host,
        packages: parse_packages(&packages)?,
        files: FileManifest::parse(&files)?,
    })
}
