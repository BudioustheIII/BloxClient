//! Parsers for Roblox's two deployment manifests:
//! `rbxPkgManifest.txt` lists packages, `rbxManifest.txt` lists every installed file.

use std::collections::{HashMap, HashSet};

use super::error::DeploymentError;

const PLAYER_PACKAGE: &str = "RobloxApp.zip";
const PLAYER_EXE: &str = "robloxplayerbeta.exe";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub md5: String,
    pub zip_size: u64,
    pub raw_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Relative path with `/` separators and the manifest's casing.
    pub path: String,
    pub md5: String,
}

#[derive(Debug, Default)]
pub struct FileManifest {
    entries: HashMap<String, FileEntry>,
    by_md5: HashMap<String, Vec<String>>,
}

fn bad(detail: impl Into<String>) -> DeploymentError {
    DeploymentError::Manifest(detail.into())
}

fn valid_md5(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn content_lines(text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.lines().map(str::trim).collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

pub fn parse_packages(text: &str) -> Result<Vec<Package>, DeploymentError> {
    let lines = content_lines(text);
    let (header, body) = lines.split_first().ok_or_else(|| bad("empty"))?;
    if *header != "v0" {
        return Err(bad(format!("unsupported format {header:?}")));
    }
    if body.is_empty() || body.len() % 4 != 0 {
        return Err(bad("unexpected package line count"));
    }

    let mut seen = HashSet::new();
    let mut packages = Vec::with_capacity(body.len() / 4);
    for group in body.chunks_exact(4) {
        let name = group[0];
        let safe_name = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && !name.starts_with('.');
        if !safe_name || !seen.insert(name.to_ascii_lowercase()) {
            return Err(bad(format!("bad package name {name:?}")));
        }
        if !valid_md5(group[1]) {
            return Err(bad(format!("bad hash for {name}")));
        }
        packages.push(Package {
            name: name.to_string(),
            md5: group[1].to_ascii_lowercase(),
            zip_size: group[2].parse().map_err(|_| bad("bad package size"))?,
            raw_size: group[3].parse().map_err(|_| bad("bad package size"))?,
        });
    }

    if !packages.iter().any(|package| package.name == PLAYER_PACKAGE) {
        return Err(bad("no player package"));
    }
    Ok(packages)
}

/// Normalizes an archive or manifest path, rejecting anything that could
/// escape the install folder.
pub fn safe_relative(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.split(['/', '\\']) {
        let invalid = part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with('.')
            || part
                .chars()
                .any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'));
        if invalid {
            return None;
        }
        parts.push(part);
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

impl FileManifest {
    pub fn parse(text: &str) -> Result<Self, DeploymentError> {
        let lines = content_lines(text);
        if lines.is_empty() || lines.len() % 2 != 0 {
            return Err(bad("unexpected file line count"));
        }

        let mut manifest = Self::default();
        for pair in lines.chunks_exact(2) {
            let path = safe_relative(pair[0]).ok_or_else(|| bad(format!("unsafe path {:?}", pair[0])))?;
            if !valid_md5(pair[1]) {
                return Err(bad(format!("bad hash for {path}")));
            }
            let md5 = pair[1].to_ascii_lowercase();
            let key = path.to_lowercase();
            manifest.by_md5.entry(md5.clone()).or_default().push(key.clone());
            if manifest.entries.insert(key, FileEntry { path, md5 }).is_some() {
                return Err(bad("duplicate file path"));
            }
        }

        if !manifest.entries.contains_key(PLAYER_EXE) {
            return Err(bad("no player executable"));
        }
        Ok(manifest)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn entries(&self) -> impl Iterator<Item = &FileEntry> {
        self.entries.values()
    }

    pub fn get(&self, path: &str) -> Option<&FileEntry> {
        self.entries.get(&path.to_lowercase())
    }

    /// Finds the folder a package extracts into. Packages don't record it, but
    /// every (name, hash) pair must match a manifest file under one shared root.
    /// `placed` holds files already claimed, for packages with identical content.
    pub fn resolve_root(&self, files: &[(String, String)], placed: &HashSet<String>) -> Option<String> {
        let (first_name, first_md5) = files.first()?;
        let first_name = first_name.to_lowercase();

        let mut roots: Vec<String> = self
            .by_md5
            .get(first_md5)?
            .iter()
            .filter_map(|path| {
                let root = path.strip_suffix(&first_name)?;
                (root.is_empty() || root.ends_with('/')).then(|| root.to_string())
            })
            .collect();
        roots.sort();
        roots.dedup();

        roots.into_iter().find(|root| {
            files.iter().all(|(name, md5)| {
                let key = format!("{root}{}", name.to_lowercase());
                !placed.contains(&key) && self.entries.get(&key).is_some_and(|entry| entry.md5 == *md5)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const C: &str = "cccccccccccccccccccccccccccccccc";

    fn packages(body: &str) -> Result<Vec<Package>, DeploymentError> {
        parse_packages(&format!("v0\r\nRobloxApp.zip\r\n{A}\r\n10\r\n20\r\n{body}"))
    }

    fn files(extra: &str) -> FileManifest {
        FileManifest::parse(&format!("RobloxPlayerBeta.exe\n{A}\n{extra}")).unwrap()
    }

    #[test]
    fn parses_package_manifest() {
        let parsed = packages(&format!("ssl.zip\n{B}\n1\n2\n\n")).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].name, "ssl.zip");
        assert_eq!(parsed[1].raw_size, 2);
    }

    #[test]
    fn rejects_bad_package_manifests() {
        assert!(parse_packages("").is_err());
        assert!(parse_packages("<html>not a manifest</html>").is_err());
        assert!(parse_packages(&format!("v1\nRobloxApp.zip\n{A}\n1\n2")).is_err());
        assert!(parse_packages(&format!("v0\nother.zip\n{A}\n1\n2")).is_err());
        assert!(packages(&format!("../evil.zip\n{B}\n1\n2")).is_err());
        assert!(packages(&format!("x.zip\nnot-a-hash\n1\n2")).is_err());
        assert!(packages(&format!("x.zip\n{B}\nbig\n2")).is_err());
        assert!(packages(&format!("RobloxApp.zip\n{B}\n1\n2")).is_err());
    }

    #[test]
    fn safe_relative_blocks_escapes() {
        assert_eq!(safe_relative(r"content\a\b.txt").as_deref(), Some("content/a/b.txt"));
        for path in [
            "", "..", "../a", r"a\..\..\b", "/abs", r"\abs", r"C:\Windows\x", "a//b", "a/./b", "a/b:stream",
            "a/b.", "a\0b",
        ] {
            assert_eq!(safe_relative(path), None, "accepted {path:?}");
        }
    }

    #[test]
    fn file_manifest_rejects_bad_input() {
        assert!(FileManifest::parse("").is_err());
        assert!(FileManifest::parse(&format!("other.exe\n{A}")).is_err());
        assert!(FileManifest::parse(&format!("RobloxPlayerBeta.exe\n{A}\n../x\n{B}")).is_err());
        assert!(FileManifest::parse(&format!("RobloxPlayerBeta.exe\n{A}\nrobloxplayerbeta.EXE\n{B}")).is_err());
    }

    #[test]
    fn resolves_package_root_by_name_and_hash() {
        let manifest = files(&format!("content\\sounds\\a.ogg\n{B}\ncontent\\sounds\\b.ogg\n{C}\n"));
        let package = vec![("a.ogg".to_string(), B.to_string()), ("b.ogg".to_string(), C.to_string())];
        assert_eq!(manifest.resolve_root(&package, &HashSet::new()).as_deref(), Some("content/sounds/"));
    }

    #[test]
    fn resolves_root_level_packages() {
        let manifest = files(&format!("WebView2Loader.dll\n{B}\n"));
        let package = vec![("WebView2Loader.dll".to_string(), B.to_string())];
        assert_eq!(manifest.resolve_root(&package, &HashSet::new()).as_deref(), Some(""));
    }

    #[test]
    fn same_named_files_are_separated_by_their_other_contents() {
        let manifest = files(&format!(
            "content\\x\\same.txt\n{B}\ncontent\\x\\one.txt\n{C}\nextra\\y\\same.txt\n{B}\nextra\\y\\two.txt\n{A}\n"
        ));
        let package = vec![("same.txt".to_string(), B.to_string()), ("two.txt".to_string(), A.to_string())];
        assert_eq!(manifest.resolve_root(&package, &HashSet::new()).as_deref(), Some("extra/y/"));
    }

    #[test]
    fn identical_packages_claim_different_roots() {
        let manifest = files(&format!("one\\same.txt\n{B}\ntwo\\same.txt\n{B}\n"));
        let package = vec![("same.txt".to_string(), B.to_string())];
        let placed = HashSet::from(["one/same.txt".to_string()]);
        assert_eq!(manifest.resolve_root(&package, &placed).as_deref(), Some("two/"));
    }

    #[test]
    fn tampered_or_partial_matches_are_rejected() {
        let manifest = files(&format!("content\\a.txt\n{B}\n"));
        let tampered = vec![("a.txt".to_string(), C.to_string())];
        assert_eq!(manifest.resolve_root(&tampered, &HashSet::new()), None);
        let extra_file = vec![("a.txt".to_string(), B.to_string()), ("evil.dll".to_string(), C.to_string())];
        assert_eq!(manifest.resolve_root(&extra_file, &HashSet::new()), None);
        let suffix_only = vec![("t.txt".to_string(), B.to_string())];
        assert_eq!(manifest.resolve_root(&suffix_only, &HashSet::new()), None);
    }
}
