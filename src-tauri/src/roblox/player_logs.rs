//! Reads which games were joined from the Roblox Player's local logs. These
//! record every session regardless of launcher, and Roblox offers no API for
//! play history without the login cookie.
//!
//! Log lines can contain sensitive session data. Only place ids, universe ids
//! and timestamps are extracted, and line contents are never stored or logged.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

const JOIN_MARKER: &[u8] = b"! Joining game '";
const JOIN_PLACE: &[u8] = b"' place ";
const LOAD_MARKER: &[u8] = b"game_join_loadtime";
const LOAD_PLACE: &[u8] = b"placeid:";
const LOAD_UNIVERSE: &[u8] = b"universeid:";
const PLAYER_LOG_TAG: &str = "_Player_";
const CRASH_LOG_TAG: &str = "CrashHandler";
const MAX_FILES: usize = 60;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LINE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogPlay {
    pub universe_id: u64,
    pub played_at: i64,
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

/// Returns the digits immediately after `marker`, and the rest of the line.
fn number_after<'a>(line: &'a [u8], marker: &[u8]) -> Option<(u64, &'a [u8])> {
    let start = find(line, marker)? + marker.len();
    let rest = &line[start..];
    let length = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    let value = std::str::from_utf8(&rest[..length]).ok()?.parse().ok()?;
    Some((value, &rest[length..]))
}

fn digits(bytes: &[u8]) -> Option<i64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Parses the `2026-11-23T11:05:19.440Z` prefix of a log line to Unix seconds.
fn line_timestamp(line: &[u8]) -> Option<i64> {
    let stamp = line.get(..19)?;
    if stamp[4] != b'-' || stamp[7] != b'-' || stamp[10] != b'T' || stamp[13] != b':' || stamp[16] != b':' {
        return None;
    }
    let (year, month, day) = (digits(&stamp[..4])?, digits(&stamp[5..7])?, digits(&stamp[8..10])?);
    let (hour, minute, second) = (digits(&stamp[11..13])?, digits(&stamp[14..16])?, digits(&stamp[17..19])?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Finds games that were joined and finished loading. A join with no load
/// record failed before the game opened, so it doesn't count as played.
pub fn parse_log(mut reader: impl BufRead) -> io::Result<Vec<LogPlay>> {
    let mut joins: Vec<(u64, i64)> = Vec::new();
    let mut universes: HashMap<u64, u64> = HashMap::new();
    let mut line = Vec::with_capacity(512);

    loop {
        line.clear();
        if reader.by_ref().take(MAX_LINE_BYTES).read_until(b'\n', &mut line)? == 0 {
            break;
        }
        if let Some(start) = find(&line, JOIN_MARKER) {
            let after = &line[start + JOIN_MARKER.len()..];
            if let (Some((place, _)), Some(time)) = (number_after(after, JOIN_PLACE), line_timestamp(&line)) {
                joins.push((place, time));
            }
        } else if let Some(start) = find(&line, LOAD_MARKER) {
            let after = &line[start..];
            if let Some((place, rest)) = number_after(after, LOAD_PLACE) {
                if let Some((universe, _)) = number_after(rest, LOAD_UNIVERSE) {
                    universes.insert(place, universe);
                }
            }
        }
    }

    Ok(joins
        .into_iter()
        .filter_map(|(place, played_at)| {
            universes.get(&place).map(|&universe_id| LogPlay { universe_id, played_at })
        })
        .collect())
}

/// Player logs only: Studio logs record test sessions, not games played.
fn is_player_log(name: &str) -> bool {
    name.ends_with(".log") && name.contains(PLAYER_LOG_TAG) && !name.contains(CRASH_LOG_TAG)
}

fn newest_player_logs(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut logs: Vec<_> = entries
        .flatten()
        .filter(|entry| is_player_log(&entry.file_name().to_string_lossy()))
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            (metadata.is_file() && metadata.len() <= MAX_FILE_BYTES).then_some((metadata.modified().ok()?, entry.path()))
        })
        .collect();
    logs.sort_by(|a, b| b.0.cmp(&a.0));
    logs.into_iter().take(MAX_FILES).map(|(_, path)| path).collect()
}

/// Unreadable files are skipped: Roblox may be writing or deleting them.
pub fn read_player_logs(folder: &Path) -> Vec<LogPlay> {
    newest_player_logs(folder)
        .into_iter()
        .filter_map(|path| File::open(path).ok())
        .filter_map(|file| parse_log(BufReader::new(file)).ok())
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const JOIN: &str = "2026-11-23T11:05:19.440Z,12.5,abc,6 [FLog::Output] ! Joining game 'a1b2c3d4-0000-4000-8000-000000000000' place 606849621 at 10.0.0.1\n";
    const LOAD: &str = "2026-11-23T11:05:25.000Z,18.1,abc,6 [FLog::GameJoinLoadTime] Report game_join_loadtime: placeid:606849621, visitid:1, universeid:245662005, status:1\n";

    fn parse(text: &str) -> Vec<LogPlay> {
        parse_log(Cursor::new(text.as_bytes().to_vec())).unwrap()
    }

    #[test]
    fn extracts_a_loaded_game() {
        assert_eq!(
            parse(&format!("noise\n{JOIN}more noise\n{LOAD}")),
            [LogPlay { universe_id: 245662005, played_at: 1_795_431_919 }]
        );
    }

    #[test]
    fn load_record_may_come_from_anywhere_in_the_file() {
        assert_eq!(parse(&format!("{LOAD}{JOIN}")).len(), 1);
    }

    #[test]
    fn failed_joins_do_not_count() {
        assert!(parse(JOIN).is_empty());
        let other_game = LOAD.replace("placeid:606849621", "placeid:111");
        assert!(parse(&format!("{JOIN}{other_game}")).is_empty());
    }

    #[test]
    fn malformed_lines_are_ignored() {
        let bad_time = JOIN.replace("2026-11-23T11", "not-a-timestampX");
        let bad_place = JOIN.replace("place 606849621", "place abc");
        let text = format!("{bad_time}{bad_place}! Joining game '\ngame_join_loadtime placeid:\n\u{0}\u{1}\n");
        assert!(parse(&text).is_empty());
    }

    #[test]
    fn non_utf8_and_oversized_lines_do_not_break_parsing() {
        let mut data = vec![0xFF, 0xFE, b'\n'];
        data.extend(vec![b'x'; (MAX_LINE_BYTES * 2) as usize]);
        data.push(b'\n');
        data.extend_from_slice(JOIN.as_bytes());
        data.extend_from_slice(LOAD.as_bytes());
        assert_eq!(parse_log(Cursor::new(data)).unwrap().len(), 1);
    }

    #[test]
    fn timestamps_convert_to_unix_time() {
        assert_eq!(line_timestamp(b"1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(line_timestamp(b"2000-03-01T00:00:00.000Z"), Some(951_868_800));
        assert_eq!(line_timestamp(b"2026-13-01T00:00:00.000Z"), None);
        assert_eq!(line_timestamp(b"short"), None);
    }

    #[test]
    fn only_player_logs_are_read() {
        assert!(is_player_log("0.748.0.7481526_20261123T110000Z_Player_ABCDE_last.log"));
        assert!(!is_player_log("0.748.0.7481526_20261123T110000Z_Player_ABCDE_CrashHandler_last.log"));
        assert!(!is_player_log("0.748.0.7481526_20261123T110000Z_Studio_ABCDE_last.log"));
        assert!(!is_player_log("RobloxPlayerInstaller_ABCDE.log"));
        assert!(!is_player_log("0.748.0_Player_ABCDE_last.txt"));
    }
}
