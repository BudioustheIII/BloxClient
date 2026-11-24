//! Read-only detection of deployments Roblox installed itself.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::model::{DeploymentOrigin, DeploymentStatus, InstalledDeployment, VersionHash};

pub const PLAYER_EXE: &str = "RobloxPlayerBeta.exe";
const STUDIO_EXE: &str = "RobloxStudioBeta.exe";
const REQUIRED: &[&str] = &[PLAYER_EXE, "RobloxPlayerBeta.dll", "AppSettings.xml", "content"];

pub fn status_of(path: &Path) -> DeploymentStatus {
    if REQUIRED.iter().all(|item| path.join(item).exists()) {
        DeploymentStatus::Ready
    } else {
        DeploymentStatus::Incomplete
    }
}

/// Lists Player installs. Studio installs, empty folders, links and folders
/// that aren't named like a version are ignored.
pub fn scan_roblox_versions(root: &Path) -> Vec<InstalledDeployment> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let version = VersionHash::parse(&name).ok()?;
            if version.as_str() != name || !entry.file_type().ok()?.is_dir() {
                return None;
            }
            let path = entry.path();
            if path.join(STUDIO_EXE).exists() || fs::read_dir(&path).ok()?.next().is_none() {
                return None;
            }

            let installed_at = fs::metadata(path.join(PLAYER_EXE))
                .and_then(|metadata| metadata.created())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|age| age.as_secs() as f64);

            Some(InstalledDeployment {
                version: name,
                status: status_of(&path),
                path: path.display().to_string(),
                origin: DeploymentOrigin::Roblox,
                installed_at,
                size_bytes: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bloxclient-scan-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn make(root: &Path, dir: &str, files: &[&str]) {
        let path = root.join(dir);
        fs::create_dir_all(&path).unwrap();
        for file in files {
            if *file == "content" {
                fs::create_dir_all(path.join(file)).unwrap();
            } else {
                fs::write(path.join(file), b"x").unwrap();
            }
        }
    }

    #[test]
    fn classifies_directories() {
        let root = temp_root("classify");
        make(&root, "version-aaaaaaaaaaaaaaaa", REQUIRED);
        make(&root, "version-bbbbbbbbbbbbbbbb", &[PLAYER_EXE]);
        make(&root, "version-cccccccccccccccc", &[STUDIO_EXE, "content"]);
        make(&root, "version-dddddddddddddddd", &[]);
        make(&root, "not-a-version", REQUIRED);
        make(&root, "VERSION-EEEEEEEEEEEEEEEE", REQUIRED);
        fs::write(root.join("version-ffffffffffffffff"), b"a file").unwrap();

        let mut found = scan_roblox_versions(&root);
        found.sort_by(|a, b| a.version.cmp(&b.version));
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].version, "version-aaaaaaaaaaaaaaaa");
        assert_eq!(found[0].status, DeploymentStatus::Ready);
        assert_eq!(found[1].version, "version-bbbbbbbbbbbbbbbb");
        assert_eq!(found[1].status, DeploymentStatus::Incomplete);
    }

    #[test]
    fn missing_root_is_empty() {
        assert!(scan_roblox_versions(Path::new(r"C:\bloxclient-no-such-folder")).is_empty());
    }
}
