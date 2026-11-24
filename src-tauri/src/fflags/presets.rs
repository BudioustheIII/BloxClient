//! Read-only recommended profiles. They only use flags from the published
//! baseline list and never touch security or anti-cheat behavior.

use super::model::{FlagEntry, FlagKind};

pub const DEFAULT_ID: &str = "preset:default";

pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    flags: &'static [(&'static str, FlagKind, &'static str)],
}

impl Preset {
    pub fn entries(&self) -> Vec<FlagEntry> {
        self.flags
            .iter()
            .map(|(name, kind, value)| FlagEntry {
                name: (*name).to_string(),
                kind: *kind,
                value: (*value).to_string(),
                enabled: true,
            })
            .collect()
    }
}

pub const PRESETS: [Preset; 3] = [
    Preset { id: DEFAULT_ID, name: "Default", flags: &[] },
    Preset {
        id: "preset:performance",
        name: "Performance",
        flags: &[
            ("DFFlagTextureQualityOverrideEnabled", FlagKind::Boolean, "true"),
            ("DFIntTextureQualityOverride", FlagKind::Number, "0"),
            ("FIntFRMMaxGrassDistance", FlagKind::Number, "0"),
            ("FIntFRMMinGrassDistance", FlagKind::Number, "0"),
        ],
    },
    Preset {
        id: "preset:quality",
        name: "Quality",
        flags: &[
            ("DFFlagTextureQualityOverrideEnabled", FlagKind::Boolean, "true"),
            ("DFIntTextureQualityOverride", FlagKind::Number, "3"),
            ("FIntDebugForceMSAASamples", FlagKind::Number, "4"),
        ],
    },
];

pub fn find(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.id == id)
}

#[cfg(test)]
mod tests {
    use super::super::validate::{check_name, is_listed, normalize_value};
    use super::*;

    #[test]
    fn presets_only_use_valid_baseline_flags() {
        for preset in &PRESETS {
            for entry in preset.entries() {
                assert!(is_listed(&entry.name), "{} isn't on the baseline list", entry.name);
                assert_eq!(check_name(&entry.name).unwrap(), entry.name);
                assert_eq!(normalize_value(&entry.name, entry.kind, &entry.value).unwrap(), entry.value);
            }
        }
    }
}
