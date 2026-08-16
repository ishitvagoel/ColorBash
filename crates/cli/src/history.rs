use std::fmt;

pub const DEFAULT_QUEUE_CAPACITY: usize = 1024;
pub const DEFAULT_RETENTION_ROWS: u64 = 200_000;
pub const DEFAULT_RETENTION_DAYS: u64 = 90;
pub const DEFAULT_QUERY_LIMIT: usize = 50;
pub const MAX_QUERY_LIMIT: usize = 500;
pub const MAX_COMMAND_BYTES: usize = 64 * 1024;
pub const SCHEMA_VERSION: i64 = 2;
pub const FUZZY_CANDIDATE_LIMIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub session_id: String,
    pub event_sequence: u64,
    pub history_number: Option<i64>,
    pub command_text: String,
    pub start_cwd: String,
    pub completed_at: String,
    pub status: i32,
    pub duration_ms: Option<u64>,
    pub host: String,
    pub user: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryErrorKind {
    Disabled,
    Open,
    Migrate,
    Write,
    Read,
    QueueFull,
    StorageFailure,
    InvalidInput,
}

impl HistoryErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Open => "open",
            Self::Migrate => "migrate",
            Self::Write => "write",
            Self::Read => "read",
            Self::QueueFull => "queue_full",
            Self::StorageFailure => "storage_failure",
            Self::InvalidInput => "invalid_input",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryError {
    kind: HistoryErrorKind,
    message: String,
}

impl HistoryError {
    pub fn new(kind: HistoryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> HistoryErrorKind {
        self.kind
    }
}

/// Applies the ADR drop rule: empty, NUL-containing, invalid-UTF-8, or
/// oversized command text is rejected without truncation.
pub fn validate_entry(entry: &HistoryEntry) -> Result<(), HistoryError> {
    if entry.command_text.is_empty() {
        return Err(HistoryError::new(
            HistoryErrorKind::InvalidInput,
            "empty command text is rejected",
        ));
    }
    if entry.command_text.len() > MAX_COMMAND_BYTES {
        return Err(HistoryError::new(
            HistoryErrorKind::InvalidInput,
            "oversized command text is rejected without truncation",
        ));
    }
    if entry.command_text.contains('\0') {
        return Err(HistoryError::new(
            HistoryErrorKind::InvalidInput,
            "NUL in command text is rejected",
        ));
    }
    Ok(())
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "history {}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for HistoryError {}

pub trait HistoryPolicy: Send + Sync {
    fn disabled(&self) -> bool;
    fn allows(&self, entry: &HistoryEntry) -> bool;
}

pub trait HistoryRecorder: Send + Sync {
    fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError>;
}

pub trait HistorySearch: Send + Sync {
    fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError>;
    fn exact_prefix(&self, prefix: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError>;
    fn by_cwd(&self, cwd: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError>;
    fn fuzzy(&self, needle: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError>;
    fn failed(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError>;
}

/// Scores `command` against `needle` for bounded fuzzy ranking (HIST-009).
/// Exact 300, prefix 200, substring 100, subsequence 50, else 0.
pub fn fuzzy_score(needle: &str, command: &str) -> u32 {
    if needle.is_empty() || command.is_empty() {
        return 0;
    }
    let needle = needle.to_ascii_lowercase();
    let command = command.to_ascii_lowercase();
    if command == needle {
        return 300;
    }
    if command.starts_with(&needle) {
        return 200;
    }
    if command.contains(&needle) {
        return 100;
    }
    if is_subsequence(&needle, &command) {
        return 50;
    }
    0
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    needle
        .chars()
        .all(|needle_ch| chars.any(|hay| hay == needle_ch))
}

pub trait HistoryControl: Send + Sync {
    fn count(&self) -> Result<u64, HistoryError>;
    fn clear(&self) -> Result<(), HistoryError>;
    fn delete(&self) -> Result<(), HistoryError>;
}

#[cfg(test)]
mod tests {
    use super::fuzzy_score;

    #[test]
    fn fuzzy_score_orders_exact_prefix_substring_then_subsequence() {
        assert_eq!(fuzzy_score("git", "git"), 300);
        assert_eq!(fuzzy_score("git", "git status"), 200);
        assert_eq!(fuzzy_score("status", "git status"), 100);
        assert_eq!(fuzzy_score("gst", "git status"), 50);
        assert_eq!(fuzzy_score("xyz", "git status"), 0);
        assert_eq!(fuzzy_score("", "git"), 0);
    }
}
