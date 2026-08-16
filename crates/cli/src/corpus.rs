//! HIST-004 seeded history corpus for tests and the 100k-row budget run.
//!
//! Compiled only into the library test build; it is not a helper runtime path.

use crate::history::HistoryEntry;

pub const CORPUS_SEED: u64 = 0x4D42_5831;
pub const CORPUS_SIZE: usize = 100_000;
pub const CWD_POOL: u64 = 200;
const TIMESTAMP_START_UNIX: u64 = 1_779_235_200; // 2026-05-20T00:00:00Z
const TIMESTAMP_SPAN_SECS: u64 = 88 * 86_400;
const SESSION_COUNT: u64 = 8;

const SHORT_COMMANDS: [&str; 7] = [
    "ls",
    "git status",
    "cd ~/src",
    "pwd",
    "echo ok",
    "git diff",
    "git log",
];

const HOSTILE_COMMANDS: [&str; 8] = [
    "'; DROP TABLE history;--",
    "' OR '1'='1",
    "git commit -m \"'; rm -rf /\"",
    "printf '\\x1b[31mred'",
    "echo $'\\x1b]0;PWN\\x07title'",
    "cmd `whoami` $(reboot) ${HOME}",
    "100%_done",
    "ls /tmp/中文/café",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorpusKind {
    Short,
    Medium,
    Long,
    Multiline,
    Hostile,
    Duplicate,
}

pub fn kind_at(index: u64) -> CorpusKind {
    match index % 100 {
        0..=54 => CorpusKind::Short,
        55..=79 => CorpusKind::Medium,
        80..=89 => CorpusKind::Long,
        90..=94 => CorpusKind::Multiline,
        95..=97 => CorpusKind::Hostile,
        _ => CorpusKind::Duplicate,
    }
}

pub fn entry_at(seed: u64, index: u64) -> HistoryEntry {
    let session_id = format!("session-{:02}", index % SESSION_COUNT);
    HistoryEntry {
        session_id,
        event_sequence: index / SESSION_COUNT,
        history_number: Some((index / SESSION_COUNT + 1) as i64),
        command_text: command_at(seed, index),
        start_cwd: cwd_at(index),
        completed_at: completed_at(index),
        status: if index % 17 == 0 { 1 } else { 0 },
        duration_ms: (index % 5 == 0).then_some((index % 2500) + 1),
        host: "bench-host".to_owned(),
        user: "bench-user".to_owned(),
        repo_root: None,
        repo_branch: None,
    }
}

pub fn cwd_at(index: u64) -> String {
    format!("/corpus/d{:03}", index % CWD_POOL)
}

pub fn command_at(seed: u64, index: u64) -> String {
    match kind_at(index) {
        CorpusKind::Short => {
            SHORT_COMMANDS[(index % SHORT_COMMANDS.len() as u64) as usize].to_owned()
        }
        CorpusKind::Medium => {
            format!("rg --hidden --glob '*.rs' query-{index} {}", cwd_at(index))
        }
        CorpusKind::Long => {
            let width = 120 + (mix64(seed, index) as usize % 3_881);
            format!("printf '{}'", "A".repeat(width))
        }
        CorpusKind::Multiline => "if true; then echo one two; fi".to_owned(),
        CorpusKind::Hostile => {
            HOSTILE_COMMANDS[(index % HOSTILE_COMMANDS.len() as u64) as usize].to_owned()
        }
        CorpusKind::Duplicate => {
            SHORT_COMMANDS[(index % SHORT_COMMANDS.len() as u64) as usize].to_owned()
        }
    }
}

pub fn to_jsonl(entry: &HistoryEntry) -> String {
    let duration = match entry.duration_ms {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    };
    let history_number = match entry.history_number {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    };
    format!(
        "{{\"session_id\":{},\"event_sequence\":{},\"history_number\":{},\
         \"command_text\":{},\"start_cwd\":{},\"completed_at\":{},\
         \"status\":{},\"duration_ms\":{},\"host\":{},\"user\":{},\
         \"repo_root\":{},\"repo_branch\":{}}}",
        json_string(&entry.session_id),
        entry.event_sequence,
        history_number,
        json_string(&entry.command_text),
        json_string(&entry.start_cwd),
        json_string(&entry.completed_at),
        entry.status,
        duration,
        json_string(&entry.host),
        json_string(&entry.user),
        json_optional_string(entry.repo_root.as_deref()),
        json_optional_string(entry.repo_branch.as_deref()),
    )
}

fn json_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => json_string(value),
        None => "null".to_owned(),
    }
}

pub fn from_jsonl(line: &str) -> Result<HistoryEntry, String> {
    let object = parse_object(line)?;
    Ok(HistoryEntry {
        session_id: required_string(&object, "session_id")?,
        event_sequence: required_u64(&object, "event_sequence")?,
        history_number: optional_i64(&object, "history_number")?,
        command_text: required_string(&object, "command_text")?,
        start_cwd: required_string(&object, "start_cwd")?,
        completed_at: required_string(&object, "completed_at")?,
        status: required_i64(&object, "status")? as i32,
        duration_ms: optional_u64(&object, "duration_ms")?,
        host: required_string(&object, "host")?,
        user: required_string(&object, "user")?,
        repo_root: optional_string(&object, "repo_root")?,
        repo_branch: optional_string(&object, "repo_branch")?,
    })
}

fn completed_at(index: u64) -> String {
    unix_to_iso_utc(TIMESTAMP_START_UNIX + (index % TIMESTAMP_SPAN_SECS))
}

fn mix64(seed: u64, index: u64) -> u64 {
    let mut z = seed.wrapping_add(index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

enum JsonValue {
    Null,
    Number(i64),
    String(String),
}

fn parse_object(line: &str) -> Result<Vec<(String, JsonValue)>, String> {
    let line = line.trim();
    let body = line
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| "JSONL line must be an object".to_owned())?;
    let mut fields = Vec::new();
    let mut rest = body.trim();
    while !rest.is_empty() {
        let (key, after_key) = parse_json_string(rest)?;
        let after_colon = after_key
            .trim_start()
            .strip_prefix(':')
            .ok_or_else(|| "expected colon after JSONL key".to_owned())?
            .trim_start();
        let (value, after_value) = parse_json_value(after_colon)?;
        fields.push((key, value));
        rest = after_value.trim_start();
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else if !rest.is_empty() {
            return Err("unexpected trailing JSONL content".to_owned());
        }
    }
    Ok(fields)
}

fn parse_json_value(input: &str) -> Result<(JsonValue, &str), String> {
    if let Some(rest) = input.strip_prefix("null") {
        return Ok((JsonValue::Null, rest));
    }
    if input.starts_with('"') {
        let (value, rest) = parse_json_string(input)?;
        return Ok((JsonValue::String(value), rest));
    }
    let end = input
        .find(|character: char| !matches!(character, '0'..='9' | '-'))
        .unwrap_or(input.len());
    let number = input[..end]
        .parse::<i64>()
        .map_err(|_| "invalid JSONL number".to_owned())?;
    Ok((JsonValue::Number(number), &input[end..]))
}

fn parse_json_string(input: &str) -> Result<(String, &str), String> {
    let mut chars = input.char_indices();
    match chars.next() {
        Some((_, '"')) => {}
        _ => return Err("expected JSONL string".to_owned()),
    }
    let mut out = String::new();
    while let Some((index, character)) = chars.next() {
        match character {
            '"' => return Ok((out, &input[index + character.len_utf8()..])),
            '\\' => {
                let Some((_, escaped)) = chars.next() else {
                    return Err("truncated JSONL escape".to_owned());
                };
                match escaped {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            let Some((_, digit)) = chars.next() else {
                                return Err("truncated JSONL unicode escape".to_owned());
                            };
                            hex.push(digit);
                        }
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| "invalid JSONL unicode escape".to_owned())?;
                        out.push(
                            char::from_u32(code)
                                .ok_or_else(|| "invalid JSONL unicode escape".to_owned())?,
                        );
                    }
                    _ => return Err("unsupported JSONL escape".to_owned()),
                }
            }
            other => out.push(other),
        }
    }
    Err("unterminated JSONL string".to_owned())
}

fn required_string(fields: &[(String, JsonValue)], key: &str) -> Result<String, String> {
    match field(fields, key)? {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(format!("JSONL field {key} must be a string")),
    }
}

fn required_u64(fields: &[(String, JsonValue)], key: &str) -> Result<u64, String> {
    match field(fields, key)? {
        JsonValue::Number(value) if *value >= 0 => Ok(*value as u64),
        _ => Err(format!("JSONL field {key} must be an unsigned integer")),
    }
}

fn required_i64(fields: &[(String, JsonValue)], key: &str) -> Result<i64, String> {
    match field(fields, key)? {
        JsonValue::Number(value) => Ok(*value),
        _ => Err(format!("JSONL field {key} must be an integer")),
    }
}

fn optional_i64(fields: &[(String, JsonValue)], key: &str) -> Result<Option<i64>, String> {
    match field(fields, key)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(value) => Ok(Some(*value)),
        _ => Err(format!("JSONL field {key} must be an integer or null")),
    }
}

fn optional_u64(fields: &[(String, JsonValue)], key: &str) -> Result<Option<u64>, String> {
    match field(fields, key)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(value) if *value >= 0 => Ok(Some(*value as u64)),
        _ => Err(format!(
            "JSONL field {key} must be an unsigned integer or null"
        )),
    }
}

fn optional_string(fields: &[(String, JsonValue)], key: &str) -> Result<Option<String>, String> {
    match fields.iter().find(|(name, _)| name == key) {
        None => Ok(None),
        Some((_, JsonValue::Null)) => Ok(None),
        Some((_, JsonValue::String(value))) => Ok(Some(value.clone())),
        Some(_) => Err(format!("JSONL field {key} must be a string or null")),
    }
}

fn field<'a>(fields: &'a [(String, JsonValue)], key: &str) -> Result<&'a JsonValue, String> {
    fields
        .iter()
        .find_map(|(name, value)| (name == key).then_some(value))
        .ok_or_else(|| format!("missing JSONL field {key}"))
}

fn unix_to_iso_utc(seconds: u64) -> String {
    let days = seconds / 86_400;
    let remainder = seconds % 86_400;
    let hours = remainder / 3_600;
    let minutes = (remainder % 3_600) / 60;
    let secs = remainder % 60;
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{secs:02}Z")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{
        HistoryControl, HistoryErrorKind, HistoryRecorder, HistorySearch, SCHEMA_VERSION,
    };
    use crate::storage::{QueuedHistoryStore, apply_schema_v1};
    use std::path::PathBuf;
    use std::thread;
    use std::time::Instant;

    fn temp_store(name: &str) -> (TempDir, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "mbx-corpus-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.sqlite3");
        (TempDir(dir), path)
    }

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn enqueue(store: &QueuedHistoryStore, entry: HistoryEntry) {
        loop {
            match store.record(entry.clone()) {
                Ok(()) => return,
                Err(error) if error.kind() == HistoryErrorKind::QueueFull => {
                    thread::yield_now();
                }
                Err(error) => panic!("record failed: {error}"),
            }
        }
    }

    fn percentile(sorted: &[u64], percentile: u64) -> u64 {
        let rank = (sorted.len() as u64 * percentile).div_ceil(100);
        let index = rank.saturating_sub(1) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    fn index_count(connection: &rusqlite::Connection, name: &str) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                [name],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn measure_query(iterations: usize, mut query: impl FnMut()) -> [u64; 3] {
        query();
        let mut samples = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let started = Instant::now();
            query();
            samples.push(started.elapsed().as_nanos() as u64);
        }
        samples.sort_unstable();
        [
            percentile(&samples, 50),
            percentile(&samples, 95),
            percentile(&samples, 99),
        ]
    }

    #[test]
    fn same_seed_emits_identical_rows() {
        let left: Vec<_> = (0..200).map(|index| entry_at(CORPUS_SEED, index)).collect();
        let right: Vec<_> = (0..200).map(|index| entry_at(CORPUS_SEED, index)).collect();
        assert_eq!(left, right);
    }

    #[test]
    fn mix_matches_hundred_slot_mapping() {
        let mut counts = [0usize; 6];
        for index in 0..10_000u64 {
            let slot = match kind_at(index) {
                CorpusKind::Short => 0,
                CorpusKind::Medium => 1,
                CorpusKind::Long => 2,
                CorpusKind::Multiline => 3,
                CorpusKind::Hostile => 4,
                CorpusKind::Duplicate => 5,
            };
            counts[slot] += 1;
        }
        assert_eq!(counts, [5500, 2500, 1000, 500, 300, 200]);
    }

    #[test]
    fn jsonl_round_trips_representative_rows() {
        for index in [0, 55, 80, 90, 95, 98] {
            let entry = entry_at(CORPUS_SEED, index);
            let parsed = from_jsonl(&to_jsonl(&entry)).expect("JSONL should parse");
            assert_eq!(parsed, entry);
        }
    }

    #[test]
    fn hostile_sql_and_control_rows_stay_inert() {
        let (dir, path) = temp_store("hostile");
        let drop_table = "'; DROP TABLE history;--";
        let injection_prefix = "' OR '1'='1";
        let control = "printf '\\x1b[31mred'";
        let cjk = "ls /tmp/中文/café";
        let percent = "100%_done";
        let hostile_cwd = "'; DROP TABLE history;--";
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            enqueue(
                &store,
                HistoryEntry {
                    session_id: "s1".to_owned(),
                    event_sequence: 1,
                    history_number: Some(1),
                    command_text: drop_table.to_owned(),
                    start_cwd: "/safe".to_owned(),
                    completed_at: "2026-08-15T10:00:00Z".to_owned(),
                    status: 0,
                    duration_ms: None,
                    host: "host".to_owned(),
                    user: "user".to_owned(),
                    repo_root: None,
                    repo_branch: None,
                },
            );
            enqueue(
                &store,
                HistoryEntry {
                    session_id: "s1".to_owned(),
                    event_sequence: 2,
                    history_number: Some(2),
                    command_text: injection_prefix.to_owned(),
                    start_cwd: hostile_cwd.to_owned(),
                    completed_at: "2026-08-15T10:00:01Z".to_owned(),
                    status: 0,
                    duration_ms: None,
                    host: "host".to_owned(),
                    user: "user".to_owned(),
                    repo_root: None,
                    repo_branch: None,
                },
            );
            enqueue(
                &store,
                HistoryEntry {
                    session_id: "s1".to_owned(),
                    event_sequence: 3,
                    history_number: Some(3),
                    command_text: control.to_owned(),
                    start_cwd: "/safe".to_owned(),
                    completed_at: "2026-08-15T10:00:02Z".to_owned(),
                    status: 0,
                    duration_ms: None,
                    host: "host".to_owned(),
                    user: "user".to_owned(),
                    repo_root: None,
                    repo_branch: None,
                },
            );
            enqueue(
                &store,
                HistoryEntry {
                    session_id: "s1".to_owned(),
                    event_sequence: 4,
                    history_number: Some(4),
                    command_text: cjk.to_owned(),
                    start_cwd: "/safe".to_owned(),
                    completed_at: "2026-08-15T10:00:03Z".to_owned(),
                    status: 0,
                    duration_ms: None,
                    host: "host".to_owned(),
                    user: "user".to_owned(),
                    repo_root: None,
                    repo_branch: None,
                },
            );
            enqueue(
                &store,
                HistoryEntry {
                    session_id: "s1".to_owned(),
                    event_sequence: 5,
                    history_number: Some(5),
                    command_text: percent.to_owned(),
                    start_cwd: "/safe".to_owned(),
                    completed_at: "2026-08-15T10:00:04Z".to_owned(),
                    status: 0,
                    duration_ms: None,
                    host: "host".to_owned(),
                    user: "user".to_owned(),
                    repo_root: None,
                    repo_branch: None,
                },
            );
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        assert_eq!(store.count().unwrap(), 5);
        let recent = store.recent(10).unwrap();
        assert_eq!(recent[4].command_text, drop_table);
        assert_eq!(recent[3].command_text, injection_prefix);
        assert_eq!(recent[2].command_text, control);
        assert_eq!(recent[1].command_text, cjk);
        assert_eq!(recent[0].command_text, percent);
        let prefix = store.exact_prefix("' OR '1'='1", 10).unwrap();
        assert_eq!(prefix.len(), 1);
        assert_eq!(prefix[0].command_text, injection_prefix);
        let percent_prefix = store.exact_prefix("100%", 10).unwrap();
        assert_eq!(percent_prefix.len(), 1);
        assert_eq!(percent_prefix[0].command_text, percent);
        let underscore = store.exact_prefix("100%_", 10).unwrap();
        assert_eq!(underscore.len(), 1);
        let by_cwd = store.by_cwd(hostile_cwd, 10).unwrap();
        assert_eq!(by_cwd.len(), 1);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='history'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
        drop(dir);
    }

    #[test]
    #[ignore = "HIST-004 case 8 100k v1→v2 migrate; run via scripts/benchmark-history-migrate.bash"]
    fn schema_v1_100k_corpus_migrates_to_v2() {
        let started = Instant::now();
        let (dir, path) = temp_store("v1-100k");
        let original_len = {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute_batch("PRAGMA journal_mode=WAL;")
                .unwrap();
            apply_schema_v1(&connection).unwrap();
            connection.execute("PRAGMA user_version = 1", []).unwrap();
            connection.execute_batch("BEGIN IMMEDIATE;").unwrap();
            for index in 0..CORPUS_SIZE as u64 {
                let entry = entry_at(CORPUS_SEED, index);
                connection
                    .execute(
                        "INSERT OR IGNORE INTO history \
                         (session_id, event_sequence, history_number, command_text, start_cwd, \
                          completed_at, status, duration_ms, host, user) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        rusqlite::params![
                            entry.session_id,
                            entry.event_sequence,
                            entry.history_number,
                            entry.command_text,
                            entry.start_cwd,
                            entry.completed_at,
                            entry.status,
                            entry.duration_ms,
                            entry.host,
                            entry.user,
                        ],
                    )
                    .unwrap();
            }
            connection.execute_batch("COMMIT;").unwrap();
            let version: i64 = connection
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .unwrap();
            assert_eq!(version, 1);
            let pre_count: i64 = connection
                .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
                .unwrap();
            assert_eq!(pre_count, CORPUS_SIZE as i64);
            std::fs::metadata(&path).unwrap().len()
        };
        let store = QueuedHistoryStore::open_with_limits(&path, 8_192, 1_000_000, 36_500).unwrap();
        drop(store);
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            let version: i64 = connection
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .unwrap();
            assert_eq!(version, SCHEMA_VERSION);
            assert_eq!(index_count(&connection, "history_prefix"), 1);
            assert_eq!(index_count(&connection, "history_prefix_completed"), 1);
            assert_eq!(index_count(&connection, "history_repo_root"), 1);
        }
        let store = QueuedHistoryStore::open_with_limits(&path, 8_192, 1_000_000, 36_500).unwrap();
        assert_eq!(store.count().unwrap(), CORPUS_SIZE as u64);
        let git = store.exact_prefix("git", 50).unwrap();
        assert!(!git.is_empty());
        assert!(
            git.windows(2)
                .all(|pair| pair[0].completed_at >= pair[1].completed_at)
        );
        assert!(path.exists());
        assert_ne!(std::fs::metadata(&path).unwrap().len(), 0);
        assert!(std::fs::metadata(&path).unwrap().len() >= original_len);
        println!(
            "area=history_migrate_v1_v2 rows={CORPUS_SIZE} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        drop(store);
        drop(dir);
    }

    #[test]
    #[ignore = "HIST-004 100k-row release benchmark; run via scripts/benchmark-history.bash"]
    fn load_100k_and_measure_query_percentiles() {
        let iterations = std::env::var("MBX_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(200);
        let (dir, path) = temp_store("100k");
        {
            let store =
                QueuedHistoryStore::open_with_limits(&path, 8_192, 1_000_000, 36_500).unwrap();
            for index in 0..CORPUS_SIZE as u64 {
                enqueue(&store, entry_at(CORPUS_SEED, index));
            }
        }
        let store = QueuedHistoryStore::open_with_limits(&path, 8_192, 1_000_000, 36_500).unwrap();
        assert_eq!(store.count().unwrap(), CORPUS_SIZE as u64);

        let recent = measure_query(iterations, || {
            let rows = store.recent(50).unwrap();
            assert_eq!(rows.len(), 50);
        });
        let selective_prefix = command_at(CORPUS_SEED, 55);
        let prefix = measure_query(iterations, || {
            let rows = store.exact_prefix(&selective_prefix, 50).unwrap();
            assert!(!rows.is_empty());
        });
        let prefix_common = measure_query(iterations, || {
            let rows = store.exact_prefix("git", 50).unwrap();
            assert!(!rows.is_empty());
        });
        let cwd = measure_query(iterations, || {
            let rows = store.by_cwd("/corpus/d000", 50).unwrap();
            assert!(!rows.is_empty());
        });

        let mut enqueue_samples = Vec::with_capacity(iterations);
        for offset in 0..iterations as u64 {
            let entry = entry_at(CORPUS_SEED.wrapping_add(1), 8_000_000 + offset);
            let started = Instant::now();
            enqueue(&store, entry);
            enqueue_samples.push(started.elapsed().as_nanos() as u64);
        }
        enqueue_samples.sort_unstable();
        let enqueue = [
            percentile(&enqueue_samples, 50),
            percentile(&enqueue_samples, 95),
            percentile(&enqueue_samples, 99),
        ];

        println!(
            "area=history_query_recent rows={CORPUS_SIZE} iterations={iterations} \
             p50_ns={} p95_ns={} p99_ns={}",
            recent[0], recent[1], recent[2]
        );
        println!(
            "area=history_query_prefix rows={CORPUS_SIZE} iterations={iterations} \
             p50_ns={} p95_ns={} p99_ns={}",
            prefix[0], prefix[1], prefix[2]
        );
        println!(
            "area=history_query_prefix_common rows={CORPUS_SIZE} iterations={iterations} \
             p50_ns={} p95_ns={} p99_ns={} note=many_match_git_not_gate",
            prefix_common[0], prefix_common[1], prefix_common[2]
        );
        println!(
            "area=history_query_cwd rows={CORPUS_SIZE} iterations={iterations} \
             p50_ns={} p95_ns={} p99_ns={}",
            cwd[0], cwd[1], cwd[2]
        );
        println!(
            "area=history_enqueue_microbench rows={CORPUS_SIZE} iterations={iterations} \
             p50_ns={} p95_ns={} p99_ns={} note=not_prompt_boundary",
            enqueue[0], enqueue[1], enqueue[2]
        );
        drop(dir);
    }
}
