use crate::history::{
    HistoryControl, HistoryEntry, HistoryError, HistoryErrorKind, HistoryRecorder, HistorySearch,
    SCHEMA_VERSION,
};
use crate::telemetry::trace_message;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DIR_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;
const BUSY_TIMEOUT_MS: u64 = 100;
const WRITER_BATCH_SIZE: usize = 32;

const SCHEMA_V1: &str = "
CREATE TABLE IF NOT EXISTS history (
    session_id      TEXT    NOT NULL,
    event_sequence  INTEGER NOT NULL,
    history_number  INTEGER,
    command_text    TEXT    NOT NULL,
    start_cwd       TEXT    NOT NULL,
    completed_at    TEXT    NOT NULL,
    status          INTEGER NOT NULL,
    duration_ms     INTEGER,
    host            TEXT    NOT NULL,
    user            TEXT    NOT NULL,
    PRIMARY KEY (session_id, event_sequence)
);
CREATE INDEX IF NOT EXISTS history_completed ON history (completed_at DESC);
CREATE INDEX IF NOT EXISTS history_prefix ON history (command_text COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS history_cwd ON history (start_cwd);
";

enum QueueMessage {
    Write(HistoryEntry),
    Shutdown,
}

struct WriterSettings {
    max_rows: u64,
    retention_days: u64,
}

pub struct QueuedHistoryStore {
    sender: SyncSender<QueueMessage>,
    store_path: PathBuf,
    writer: Option<thread::JoinHandle<()>>,
}

impl QueuedHistoryStore {
    pub fn open(path: &Path, queue_capacity: usize) -> Result<Self, HistoryError> {
        Self::open_with_limits(
            path,
            queue_capacity,
            env_u64(
                "MBX_HISTORY_RETENTION_ROWS",
                crate::history::DEFAULT_RETENTION_ROWS,
            ),
            env_u64(
                "MBX_HISTORY_RETENTION_DAYS",
                crate::history::DEFAULT_RETENTION_DAYS,
            ),
        )
    }

    pub(crate) fn open_with_limits(
        path: &Path,
        queue_capacity: usize,
        max_rows: u64,
        retention_days: u64,
    ) -> Result<Self, HistoryError> {
        let store_path = path.to_path_buf();
        create_store_dir(&store_path)?;
        let connection = open_connection(&store_path)?;
        let (sender, receiver) = mpsc::sync_channel(queue_capacity);
        let settings = WriterSettings {
            max_rows,
            retention_days,
        };
        let writer = thread::spawn(move || writer_loop(receiver, connection, settings));
        Ok(Self {
            sender,
            store_path,
            writer: Some(writer),
        })
    }

    pub fn open_default(queue_capacity: usize) -> Result<Self, HistoryError> {
        Self::open(&default_store_path(), queue_capacity)
    }
}

fn writer_loop(
    receiver: Receiver<QueueMessage>,
    connection: rusqlite::Connection,
    settings: WriterSettings,
) {
    let mut pending = 0usize;
    while let Ok(message) = receiver.recv() {
        match message {
            QueueMessage::Write(entry) => {
                if pending == 0 && connection.execute_batch("BEGIN IMMEDIATE;").is_err() {
                    trace_history_failure(&HistoryError::new(
                        HistoryErrorKind::Write,
                        "writer could not begin a batch",
                    ));
                    continue;
                }
                match insert(&connection, &entry) {
                    Ok(()) => {
                        pending += 1;
                        if pending >= WRITER_BATCH_SIZE {
                            if connection.execute_batch("COMMIT;").is_err() {
                                trace_history_failure(&HistoryError::new(
                                    HistoryErrorKind::Write,
                                    "writer could not commit a batch",
                                ));
                                let _ = connection.execute_batch("ROLLBACK;");
                                pending = 0;
                            } else {
                                pending = 0;
                                let _ = prune(&connection, &settings);
                            }
                        }
                    }
                    Err(error) => {
                        trace_history_failure(&error);
                        let _ = connection.execute_batch("ROLLBACK;");
                        pending = 0;
                    }
                }
            }
            QueueMessage::Shutdown => {
                if pending > 0 && connection.execute_batch("COMMIT;").is_err() {
                    trace_history_failure(&HistoryError::new(
                        HistoryErrorKind::Write,
                        "writer could not commit a partial batch",
                    ));
                    let _ = connection.execute_batch("ROLLBACK;");
                }
                let _ = prune(&connection, &settings);
                break;
            }
        }
    }
}

impl Drop for QueuedHistoryStore {
    fn drop(&mut self) {
        let _ = self.sender.send(QueueMessage::Shutdown);
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

impl HistoryRecorder for QueuedHistoryStore {
    fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError> {
        crate::history::validate_entry(&entry)?;
        match self.sender.try_send(QueueMessage::Write(entry)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(HistoryError::new(
                HistoryErrorKind::QueueFull,
                "queue is full; record dropped",
            )),
            Err(TrySendError::Disconnected(_)) => Err(HistoryError::new(
                HistoryErrorKind::StorageFailure,
                "writer is stopped",
            )),
        }
    }
}

impl HistorySearch for QueuedHistoryStore {
    fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_connection(&self.store_path)?;
        query(
            &connection,
            "SELECT session_id, event_sequence, history_number, command_text, start_cwd, \
             completed_at, status, duration_ms, host, user \
             FROM history ORDER BY completed_at DESC, event_sequence DESC LIMIT ?1",
            &[limit.to_string()],
        )
    }

    fn exact_prefix(&self, prefix: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_connection(&self.store_path)?;
        let escaped = escape_like(prefix);
        let pattern = format!("{escaped}%");
        if escaped == prefix {
            query(
                &connection,
                "SELECT session_id, event_sequence, history_number, command_text, start_cwd, \
                 completed_at, status, duration_ms, host, user \
                 FROM history WHERE command_text LIKE ?1 \
                 ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2",
                &[pattern, limit.to_string()],
            )
        } else {
            query(
                &connection,
                "SELECT session_id, event_sequence, history_number, command_text, start_cwd, \
                 completed_at, status, duration_ms, host, user \
                 FROM history WHERE command_text LIKE ?1 ESCAPE '\\' \
                 ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2",
                &[pattern, limit.to_string()],
            )
        }
    }

    fn by_cwd(&self, cwd: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_connection(&self.store_path)?;
        query(
            &connection,
            "SELECT session_id, event_sequence, history_number, command_text, start_cwd, \
             completed_at, status, duration_ms, host, user \
             FROM history WHERE start_cwd = ?1 \
             ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2",
            &[cwd.to_owned(), limit.to_string()],
        )
    }
}

impl HistoryControl for QueuedHistoryStore {
    fn count(&self) -> Result<u64, HistoryError> {
        let connection = open_connection(&self.store_path)?;
        connection
            .query_row("SELECT COUNT(*) FROM history", [], |row| {
                row.get::<_, u64>(0)
            })
            .map_err(history_error(HistoryErrorKind::Read))
    }

    fn clear(&self) -> Result<(), HistoryError> {
        let connection = open_connection(&self.store_path)?;
        connection
            .execute("DELETE FROM history", [])
            .map_err(history_error(HistoryErrorKind::Write))?;
        Ok(())
    }

    fn delete(&self) -> Result<(), HistoryError> {
        let mut paths = vec![self.store_path.clone()];
        paths.push(self.store_path.with_file_name(format!(
            "{}-wal",
            self.store_path.file_name().map(|name| name.to_string_lossy()).unwrap_or_default()
        )));
        paths.push(self.store_path.with_file_name(format!(
            "{}-shm",
            self.store_path.file_name().map(|name| name.to_string_lossy()).unwrap_or_default()
        )));
        for path in paths {
            let _ = fs::remove_file(path);
        }
        Ok(())
    }
}

pub fn default_store_path() -> PathBuf {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                let mut path = PathBuf::from(home);
                path.push(".local/share");
                path
            })
        })
        .unwrap_or_else(|| PathBuf::from("."));
    let mut path = data_home;
    path.push("mbx");
    path.push("history.sqlite3");
    path
}

fn create_store_dir(store_path: &Path) -> Result<(), HistoryError> {
    let dir = store_path
        .parent()
        .ok_or_else(|| HistoryError::new(HistoryErrorKind::Open, "store path has no parent"))?;
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(io_error(HistoryErrorKind::Open))?;
    }
    fs::set_permissions(dir, fs::Permissions::from_mode(DIR_MODE))
        .map_err(io_error(HistoryErrorKind::Open))
}

fn open_connection(store_path: &Path) -> Result<rusqlite::Connection, HistoryError> {
    let connection =
        rusqlite::Connection::open(store_path).map_err(history_error(HistoryErrorKind::Open))?;
    fs::set_permissions(store_path, fs::Permissions::from_mode(FILE_MODE))
        .map_err(io_error(HistoryErrorKind::Open))?;
    connection
        .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))
        .map_err(history_error(HistoryErrorKind::Open))?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(history_error(HistoryErrorKind::Open))?;
    migrate(&connection)?;
    Ok(connection)
}

fn migrate(connection: &rusqlite::Connection) -> Result<(), HistoryError> {
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(history_error(HistoryErrorKind::Migrate))?;
    if version < 1 {
        connection
            .execute_batch(SCHEMA_V1)
            .map_err(history_error(HistoryErrorKind::Migrate))?;
        connection
            .execute(&format!("PRAGMA user_version = {SCHEMA_VERSION}"), [])
            .map_err(history_error(HistoryErrorKind::Migrate))?;
    }
    Ok(())
}

fn insert(connection: &rusqlite::Connection, entry: &HistoryEntry) -> Result<(), HistoryError> {
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
        .map_err(history_error(HistoryErrorKind::Write))?;
    Ok(())
}

fn prune(connection: &rusqlite::Connection, settings: &WriterSettings) -> Result<(), HistoryError> {
    let cutoff = now_utc_iso_days_ago(settings.retention_days);
    connection
        .execute("DELETE FROM history WHERE completed_at < ?1", [cutoff])
        .map_err(history_error(HistoryErrorKind::Write))?;
    let count: u64 = connection
        .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
        .map_err(history_error(HistoryErrorKind::Write))?;
    if count <= settings.max_rows {
        return Ok(());
    }
    connection
        .execute(
            "DELETE FROM history WHERE rowid IN ( \
             SELECT rowid FROM history \
             ORDER BY completed_at DESC, event_sequence DESC \
             LIMIT -1 OFFSET ?1)",
            [settings.max_rows],
        )
        .map_err(history_error(HistoryErrorKind::Write))?;
    Ok(())
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn now_utc_iso_days_ago(days: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let cutoff = now.saturating_sub(days * 86_400);
    unix_to_iso_utc(cutoff)
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

fn query(
    connection: &rusqlite::Connection,
    sql: &str,
    params: &[String],
) -> Result<Vec<HistoryEntry>, HistoryError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(history_error(HistoryErrorKind::Read))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(HistoryEntry {
                session_id: row.get(0)?,
                event_sequence: row.get(1)?,
                history_number: row.get(2)?,
                command_text: row.get(3)?,
                start_cwd: row.get(4)?,
                completed_at: row.get(5)?,
                status: row.get(6)?,
                duration_ms: row.get(7)?,
                host: row.get(8)?,
                user: row.get(9)?,
            })
        })
        .map_err(history_error(HistoryErrorKind::Read))?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(history_error(HistoryErrorKind::Read))?);
    }
    Ok(entries)
}

fn history_error(kind: HistoryErrorKind) -> impl Fn(rusqlite::Error) -> HistoryError {
    move |error| HistoryError::new(kind, error.to_string())
}

fn io_error(kind: HistoryErrorKind) -> impl Fn(std::io::Error) -> HistoryError {
    move |error| HistoryError::new(kind, error.to_string())
}

fn trace_history_failure(error: &HistoryError) {
    trace_message(&history_failure_diagnostic(error));
}

fn history_failure_diagnostic(error: &HistoryError) -> String {
    format!("event=history_storage_error kind={}", error.kind().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryControl, HistoryRecorder, HistorySearch};
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;

    fn history_env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn temp_store(name: &str) -> (tempfile_dir::TempDir, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "mbx-storage-{}-{name}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.sqlite3");
        (tempfile_dir::TempDir::new(dir), path)
    }

    fn entry(session_id: &str, sequence: u64, command: &str, cwd: &str, at: &str) -> HistoryEntry {
        HistoryEntry {
            session_id: session_id.to_owned(),
            event_sequence: sequence,
            history_number: Some(sequence as i64),
            command_text: command.to_owned(),
            start_cwd: cwd.to_owned(),
            completed_at: at.to_owned(),
            status: 0,
            duration_ms: None,
            host: "host".to_owned(),
            user: "user".to_owned(),
        }
    }

    #[test]
    fn schema_creates_tables_and_migrations_run_once() {
        let (dir, path) = temp_store("schema");
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        drop(store);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='history'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 1);
        drop(connection);
        drop(dir);
    }

    #[test]
    fn insert_and_idempotent_retry_are_single_rows() {
        let (dir, path) = temp_store("idem");
        {
            let store = QueuedHistoryStore::open(&path, 8).unwrap();
            let first = entry("s1", 1, "echo one", "/work", "2026-08-15T10:00:00Z");
            store.record(first.clone()).unwrap();
            store.record(first).unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        store.clear().unwrap();
        assert_eq!(store.count().unwrap(), 0);
        drop(dir);
    }

    #[test]
    fn store_files_are_user_only() {
        let (dir, path) = temp_store("perms");
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        drop(store);
        let dir_mode = std::fs::metadata(dir.path()).unwrap().permissions().mode();
        let file_mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(dir_mode & 0o777, DIR_MODE);
        assert_eq!(file_mode & 0o777, FILE_MODE);
        drop(dir);
    }

    #[test]
    fn writer_drops_overflow_without_blocking() {
        let (dir, path) = temp_store("overflow");
        let store = QueuedHistoryStore::open(&path, 2).unwrap();
        for sequence in 0..4 {
            let result = store.record(entry("s1", sequence, "cmd", "/w", "2026-08-15T10:00:00Z"));
            assert!(result.is_ok() || result.unwrap_err().kind() == HistoryErrorKind::QueueFull);
        }
        drop(store);
        drop(dir);
    }

    #[test]
    fn queries_return_bounded_recent_prefix_and_cwd() {
        let (dir, path) = temp_store("queries");
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            store
                .record(entry(
                    "s1",
                    1,
                    "git status",
                    "/repo",
                    "2026-08-15T10:00:00Z",
                ))
                .unwrap();
            store
                .record(entry("s1", 2, "git log", "/repo", "2026-08-15T10:00:01Z"))
                .unwrap();
            store
                .record(entry("s2", 1, "ls -la", "/home", "2026-08-15T10:00:02Z"))
                .unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        let recent = store.recent(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].command_text, "ls -la");
        let prefix = store.exact_prefix("git", 10).unwrap();
        assert_eq!(prefix.len(), 2);
        let by_cwd = store.by_cwd("/repo", 10).unwrap();
        assert_eq!(by_cwd.len(), 2);
        drop(dir);
    }

    #[test]
    fn like_metacharacters_in_prefix_are_literal() {
        let (dir, path) = temp_store("like");
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            store
                .record(entry("s1", 1, "100% done", "/w", "2026-08-15T10:00:00Z"))
                .unwrap();
            store
                .record(entry("s1", 2, "100X done", "/w", "2026-08-15T10:00:01Z"))
                .unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        let percent = store.exact_prefix("100%", 10).unwrap();
        assert_eq!(percent.len(), 1);
        assert_eq!(percent[0].command_text, "100% done");
        drop(dir);
    }

    #[test]
    fn retention_prunes_old_rows_and_caps_total() {
        let (dir, path) = temp_store("retention");
        {
            let store = QueuedHistoryStore::open_with_limits(&path, 32, 3, 1).unwrap();
            for sequence in 0..5 {
                store
                    .record(entry(
                        "s1",
                        sequence,
                        &format!("cmd {sequence}"),
                        "/w",
                        "2026-08-15T10:00:00Z",
                    ))
                    .unwrap();
            }
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        assert!(store.count().unwrap() <= 3);
        drop(dir);
    }

    #[test]
    fn default_store_path_resolves_xdg_then_home() {
        let _guard = history_env_lock();
        unsafe { std::env::set_var("XDG_DATA_HOME", "/xdg/data") };
        assert_eq!(
            default_store_path(),
            PathBuf::from("/xdg/data/mbx/history.sqlite3")
        );
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
        unsafe { std::env::set_var("HOME", "/home/tester") };
        assert_eq!(
            default_store_path(),
            PathBuf::from("/home/tester/.local/share/mbx/history.sqlite3")
        );
        unsafe { std::env::remove_var("HOME") };
    }

    #[test]
    fn writer_thread_persists_batches_until_shutdown() {
        let (dir, path) = temp_store("writer-batches");
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            for sequence in 0..4 {
                store
                    .record(entry("s1", sequence, "cmd", "/w", "2026-08-15T10:00:00Z"))
                    .unwrap();
            }
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        assert_eq!(store.count().unwrap(), 4);
        drop(dir);
    }

    #[test]
    fn row_cap_prune_keeps_every_row_under_the_limit() {
        let (dir, path) = temp_store("under-cap");
        {
            let store = QueuedHistoryStore::open(&path, 64).unwrap();
            for sequence in 0..40 {
                store
                    .record(entry(
                        "s1",
                        sequence,
                        &format!("cmd {sequence}"),
                        "/w",
                        "2026-08-15T10:00:00Z",
                    ))
                    .unwrap();
            }
        }
        let store = QueuedHistoryStore::open(&path, 64).unwrap();
        assert_eq!(store.count().unwrap(), 40);
        drop(dir);
    }

    #[test]
    fn drop_rule_rejects_empty_nul_and_oversized() {
        use crate::history::validate_entry;
        let (dir, path) = temp_store("drop-rule");
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        let empty = entry("s1", 1, "", "/w", "2026-08-15T10:00:00Z");
        assert_eq!(
            validate_entry(&empty).unwrap_err().kind(),
            HistoryErrorKind::InvalidInput
        );
        let valid = HistoryEntry {
            command_text: "echo ok".to_owned(),
            ..empty
        };
        assert!(validate_entry(&valid).is_ok());
        assert!(
            store
                .record(HistoryEntry {
                    command_text: "has\0nul".to_owned(),
                    ..valid.clone()
                })
                .unwrap_err()
                .kind()
                == HistoryErrorKind::InvalidInput
        );
        assert!(
            store
                .record(HistoryEntry {
                    command_text: "x".repeat(crate::history::MAX_COMMAND_BYTES + 1),
                    ..valid
                })
                .unwrap_err()
                .kind()
                == HistoryErrorKind::InvalidInput
        );
        drop(dir);
    }

    #[test]
    fn storage_failure_diagnostic_exposes_only_the_typed_kind() {
        let error = HistoryError::new(
            HistoryErrorKind::StorageFailure,
            "secret-command-text must stay private",
        );
        let diagnostic = history_failure_diagnostic(&error);

        assert_eq!(
            diagnostic,
            "event=history_storage_error kind=storage_failure"
        );
        assert!(!diagnostic.contains("secret-command-text"));
    }
}

#[cfg(test)]
mod tempfile_dir {
    use std::path::PathBuf;

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(path: PathBuf) -> Self {
            Self(path)
        }

        pub fn path(&self) -> &PathBuf {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
