//! Flag validation, import parsing and honest compatibility warnings.
//!
//! Roblox's live allowlist is a signed list delivered to the client, so it
//! can't be checked from here. `BASELINE` is the list from Roblox's September
//! 2025 announcement, as reproduced by two independent third-party pages. It
//! can only show a flag was allowed then, never that it works now.

use std::collections::HashSet;
use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::{Number, Value};

use super::error::FlagError;
use super::model::{FlagEntry, FlagKind, FlagView};

pub const MAX_FLAGS: usize = 2_000;
const MAX_IMPORT_BYTES: usize = 1_000_000;
const MAX_NAME_CHARS: usize = 200;
const MAX_VALUE_CHARS: usize = 4_096;

const BASELINE: [&str; 18] = [
    "DFIntCSGLevelOfDetailSwitchingDistance",
    "DFIntCSGLevelOfDetailSwitchingDistanceL12",
    "DFIntCSGLevelOfDetailSwitchingDistanceL23",
    "DFIntCSGLevelOfDetailSwitchingDistanceL34",
    "FFlagHandleAltEnterFullscreenManually",
    "DFFlagTextureQualityOverrideEnabled",
    "DFIntTextureQualityOverride",
    "FIntDebugForceMSAASamples",
    "DFFlagDisableDPIScale",
    "FFlagDebugGraphicsPreferD3D11",
    "FFlagDebugSkyGray",
    "DFFlagDebugPauseVoxelizer",
    "DFIntDebugFRMQualityLevelOverride",
    "FIntFRMMaxGrassDistance",
    "FIntFRMMinGrassDistance",
    "FFlagDebugGraphicsPreferVulkan",
    "FFlagDebugGraphicsPreferOpenGL",
    "FIntGrassMovementReducedMotionFactor",
];

/// Flags aimed at Roblox's security systems are never accepted.
const BLOCKED_TERMS: [&str; 5] = ["hyperion", "byfron", "anticheat", "anti_cheat", "tamper"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Expected {
    Boolean,
    Integer,
    Text,
}

const PREFIXES: [(&str, Expected); 11] = [
    ("DFFlag", Expected::Boolean),
    ("SFFlag", Expected::Boolean),
    ("FFlag", Expected::Boolean),
    ("DFInt", Expected::Integer),
    ("SFInt", Expected::Integer),
    ("FInt", Expected::Integer),
    ("DFLog", Expected::Integer),
    ("FLog", Expected::Integer),
    ("DFString", Expected::Text),
    ("SFString", Expected::Text),
    ("FString", Expected::Text),
];

fn expected_type(name: &str) -> Option<Expected> {
    let expected = PREFIXES
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map(|(_, expected)| *expected)?;
    // Filter variants hold a "value;id;id" list whatever the base type is.
    if name.ends_with("_PlaceFilter") || name.ends_with("_DataCenterFilter") {
        return Some(Expected::Text);
    }
    Some(expected)
}

pub fn is_listed(name: &str) -> bool {
    BASELINE.iter().any(|listed| listed.eq_ignore_ascii_case(name))
}

pub fn check_name(raw: &str) -> Result<String, FlagError> {
    let name = raw.trim();
    let mut chars = name.chars();
    let starts_well = chars.next().is_some_and(|first| first.is_ascii_alphabetic());
    let valid = starts_well
        && name.chars().count() <= MAX_NAME_CHARS
        && chars.all(|item| item.is_ascii_alphanumeric() || item == '_');
    if !valid {
        return Err(FlagError::InvalidName);
    }
    let lower = name.to_ascii_lowercase();
    if BLOCKED_TERMS.iter().any(|term| lower.contains(term)) {
        return Err(FlagError::Blocked(name.to_string()));
    }
    Ok(name.to_string())
}

/// Returns the value in canonical form, or an error if it doesn't fit its kind.
pub fn normalize_value(name: &str, kind: FlagKind, raw: &str) -> Result<String, FlagError> {
    let invalid = || FlagError::InvalidValue(name.to_string());
    if raw.chars().count() > MAX_VALUE_CHARS {
        return Err(invalid());
    }
    match kind {
        FlagKind::String => Ok(raw.to_string()),
        FlagKind::Boolean => match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Ok("true".into()),
            "false" => Ok("false".into()),
            _ => Err(invalid()),
        },
        FlagKind::Number => {
            let text = raw.trim();
            if let Ok(integer) = text.parse::<i64>() {
                return Ok(integer.to_string());
            }
            if !text.contains(['.', 'e', 'E']) {
                return Err(invalid());
            }
            let number: f64 = text.parse().map_err(|_| invalid())?;
            if number.is_finite() {
                Ok(number.to_string())
            } else {
                Err(invalid())
            }
        }
    }
}

/// The value as it appears in `ClientAppSettings.json`.
pub fn to_json(entry: &FlagEntry) -> Value {
    match entry.kind {
        FlagKind::String => Value::String(entry.value.clone()),
        FlagKind::Boolean => Value::Bool(entry.value == "true"),
        FlagKind::Number => match entry.value.parse::<i64>() {
            Ok(integer) => Value::from(integer),
            Err(_) => entry
                .value
                .parse::<f64>()
                .ok()
                .and_then(Number::from_f64)
                .map_or_else(|| Value::String(entry.value.clone()), Value::Number),
        },
    }
}

fn type_fits(expected: Expected, kind: FlagKind, value: &str) -> bool {
    match (expected, kind) {
        (Expected::Boolean, FlagKind::Boolean) | (Expected::Text, FlagKind::String) => true,
        (Expected::Boolean, FlagKind::String) => {
            value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
        }
        (Expected::Integer, FlagKind::Number | FlagKind::String) => value.trim().parse::<i64>().is_ok(),
        _ => false,
    }
}

pub fn view(entry: &FlagEntry) -> FlagView {
    let listed = is_listed(&entry.name);
    let mut warnings = Vec::new();
    if !listed {
        warnings.push(
            "Not on the allowlist Roblox published. Roblox will probably ignore it, and it may have been removed."
                .to_string(),
        );
    }
    match expected_type(&entry.name) {
        None => warnings.push("The name doesn't start with a known Roblox flag prefix.".to_string()),
        Some(expected) if !type_fits(expected, entry.kind, &entry.value) => {
            let wanted = match expected {
                Expected::Boolean => "true or false",
                Expected::Integer => "a whole number",
                Expected::Text => "text",
            };
            warnings.push(format!("Flags with this prefix normally hold {wanted}."));
        }
        Some(_) => {}
    }
    FlagView {
        name: entry.name.clone(),
        kind: entry.kind,
        value: entry.value.clone(),
        enabled: entry.enabled,
        listed,
        warnings,
    }
}

/// Keeps every key in order, because a normal JSON map silently drops repeats.
struct OrderedEntries(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for OrderedEntries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = OrderedEntries;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object of flag names and values")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Value>()? {
                    entries.push(entry);
                }
                Ok(OrderedEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

/// Parses a `ClientAppSettings.json` style object, rejecting it as a whole if
/// any entry is invalid so a partial import can never be saved.
pub fn parse_import(text: &str) -> Result<Vec<FlagEntry>, FlagError> {
    if text.len() > MAX_IMPORT_BYTES {
        return Err(FlagError::TooLarge);
    }
    let text = text.trim_start_matches('\u{feff}');
    let OrderedEntries(items) =
        serde_json::from_str(text).map_err(|error| FlagError::InvalidJson(error.to_string()))?;
    if items.len() > MAX_FLAGS {
        return Err(FlagError::TooLarge);
    }

    let mut seen = HashSet::with_capacity(items.len());
    let mut flags = Vec::with_capacity(items.len());
    for (raw_name, value) in items {
        let name = check_name(&raw_name)?;
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(FlagError::Duplicate(name));
        }
        let (kind, raw) = match &value {
            Value::Bool(flag) => (FlagKind::Boolean, flag.to_string()),
            Value::String(text) => (FlagKind::String, text.clone()),
            Value::Number(number) if number.is_i64() || number.is_f64() => (FlagKind::Number, number.to_string()),
            _ => return Err(FlagError::InvalidValue(name)),
        };
        let value = normalize_value(&name, kind, &raw)?;
        flags.push(FlagEntry { name, kind, value, enabled: true });
    }
    Ok(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: FlagKind, value: &str) -> FlagEntry {
        FlagEntry { name: name.into(), kind, value: value.into(), enabled: true }
    }

    #[test]
    fn names_are_validated() {
        assert_eq!(check_name("  FFlagGood_1 ").unwrap(), "FFlagGood_1");
        for bad in ["", "1FFlag", "FFlag Bad", "FFlag-Bad", "FFlag\"x", &"F".repeat(201)] {
            assert!(check_name(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn security_related_flags_are_refused() {
        for name in ["FFlagDisableHyperion", "DFFlagBYFRONOff", "FIntAntiCheatLevel", "FFlagNoTamperCheck"] {
            assert!(matches!(check_name(name), Err(FlagError::Blocked(_))), "accepted {name}");
        }
    }

    #[test]
    fn values_are_normalized_by_kind() {
        assert_eq!(normalize_value("F", FlagKind::Boolean, " TRUE ").unwrap(), "true");
        assert_eq!(normalize_value("F", FlagKind::Number, " 042 ").unwrap(), "42");
        assert_eq!(normalize_value("F", FlagKind::Number, "1.5").unwrap(), "1.5");
        assert_eq!(normalize_value("F", FlagKind::String, " keep spaces ").unwrap(), " keep spaces ");
        for (kind, bad) in [
            (FlagKind::Boolean, "yes"),
            (FlagKind::Number, "abc"),
            (FlagKind::Number, "99999999999999999999"),
            (FlagKind::Number, "1e999"),
            (FlagKind::Number, "NaN"),
            (FlagKind::Number, "inf"),
        ] {
            assert!(normalize_value("F", kind, bad).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn json_output_keeps_value_types() {
        assert_eq!(to_json(&entry("A", FlagKind::Boolean, "true")), Value::Bool(true));
        assert_eq!(to_json(&entry("A", FlagKind::Number, "7")), Value::from(7));
        assert_eq!(to_json(&entry("A", FlagKind::Number, "1.5")), Value::from(1.5));
        assert_eq!(to_json(&entry("A", FlagKind::String, "7")), Value::String("7".into()));
    }

    #[test]
    fn import_reads_all_types_and_a_byte_order_mark() {
        let flags = parse_import("\u{feff}{\"FFlagA\":true,\"FIntB\":5,\"FStringC\":\"x\",\"DFIntD\":\"10\"}").unwrap();
        assert_eq!(flags.len(), 4);
        assert_eq!(flags[0].kind, FlagKind::Boolean);
        assert_eq!(flags[1].kind, FlagKind::Number);
        assert_eq!(flags[3].kind, FlagKind::String);
    }

    #[test]
    fn import_rejects_duplicates_even_with_different_case() {
        for text in ["{\"FFlagA\":true,\"FFlagA\":false}", "{\"FFlagA\":true,\"fflaga\":false}"] {
            assert!(matches!(parse_import(text), Err(FlagError::Duplicate(_))));
        }
    }

    #[test]
    fn import_rejects_bad_shapes() {
        for text in ["", "[]", "5", "{\"FFlagA\":null}", "{\"FFlagA\":{}}", "{\"FFlagA\":[1]}", "{\"FFlagA\":true} x", "{\"FFlagA\":18446744073709551615}"] {
            assert!(parse_import(text).is_err(), "accepted {text}");
        }
    }

    #[test]
    fn one_bad_entry_rejects_the_whole_import() {
        assert!(matches!(
            parse_import("{\"FFlagGood\":true,\"FFlagDisableHyperion\":true}"),
            Err(FlagError::Blocked(_))
        ));
    }

    #[test]
    fn listed_flags_are_not_warned_about() {
        let shown = view(&entry("DFIntTextureQualityOverride", FlagKind::Number, "2"));
        assert!(shown.listed);
        assert!(shown.warnings.is_empty());
    }

    #[test]
    fn unlisted_and_mismatched_flags_are_warned_about() {
        let unlisted = view(&entry("DFIntTaskSchedulerTargetFps", FlagKind::Number, "240"));
        assert!(!unlisted.listed);
        assert_eq!(unlisted.warnings.len(), 1);

        let mismatch = view(&entry("DFIntTextureQualityOverride", FlagKind::Boolean, "true"));
        assert!(mismatch.warnings[0].contains("whole number"));

        let unknown = view(&entry("SomethingElse", FlagKind::String, "x"));
        assert_eq!(unknown.warnings.len(), 2);
    }

    #[test]
    fn string_encoded_values_fit_their_prefix() {
        assert!(view(&entry("FFlagDebugSkyGray", FlagKind::String, "True")).warnings.is_empty());
        assert!(view(&entry("FIntFRMMaxGrassDistance", FlagKind::String, "0")).warnings.is_empty());
    }

    #[test]
    fn filter_variants_may_hold_text() {
        let shown = view(&entry("FFlagThing_PlaceFilter", FlagKind::String, "True;123"));
        assert_eq!(shown.warnings.len(), 1, "only the allowlist warning is expected");
    }
}
