use crate::history::{
    HistoryControl, HistoryEntry, HistoryError, HistoryErrorKind, HistoryRecorder, HistorySearch,
    SCHEMA_VERSION, sanitize_repo_branch, sanitize_repo_root,
};
use crate::provider::{
    NullRepositoryContextProvider, RepositoryContext, RepositoryContextProvider,
};
use crate::telemetry::trace_message;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::OpenFlags;

const DIR_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;
const BUSY_TIMEOUT_MS: u64 = 100;
const MIGRATE_BUSY_DEADLINE_MS: u64 = 2_000;
const WRITER_BATCH_SIZE: usize = 32;
const REPO_CONTEXT_CACHE_CAPACITY: usize = 128;
const REPO_CONTEXT_CACHE_TTL: Duration = Duration::from_secs(1);

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

const SCHEMA_V2_INDEX: &str = "
CREATE INDEX IF NOT EXISTS history_prefix_completed
    ON history (command_text COLLATE NOCASE, completed_at DESC, event_sequence DESC);
";

const SCHEMA_V3_COLUMNS: &str = "
ALTER TABLE history ADD COLUMN repo_root TEXT;
ALTER TABLE history ADD COLUMN repo_branch TEXT;
CREATE INDEX IF NOT EXISTS history_repo_root ON history (repo_root);
";

const HISTORY_COLUMNS: &str = "session_id, event_sequence, history_number, command_text, \
     start_cwd, completed_at, status, duration_ms, host, user, repo_root, repo_branch";

const EXACT_PREFIX_SQL: &str = "SELECT session_id, event_sequence, history_number, command_text, \
     start_cwd, completed_at, status, duration_ms, host, user, repo_root, repo_branch \
     FROM history INDEXED BY history_prefix_completed \
     WHERE command_text COLLATE NOCASE LIKE ?1 \
     ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2";

const EXACT_PREFIX_ESCAPE_SQL: &str = "SELECT session_id, event_sequence, history_number, \
     command_text, start_cwd, completed_at, status, duration_ms, host, user, repo_root, repo_branch \
     FROM history INDEXED BY history_prefix_completed \
     WHERE command_text COLLATE NOCASE LIKE ?1 ESCAPE '\\' \
     ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2";

const EXACT_PREFIX_CWD_SQL: &str = "SELECT session_id, event_sequence, history_number, \
     command_text, start_cwd, completed_at, status, duration_ms, host, user, repo_root, repo_branch \
     FROM history \
     WHERE command_text COLLATE NOCASE LIKE ?1 AND start_cwd = ?2 \
     ORDER BY completed_at DESC, event_sequence DESC LIMIT ?3";

const EXACT_PREFIX_CWD_ESCAPE_SQL: &str = "SELECT session_id, event_sequence, history_number, \
     command_text, start_cwd, completed_at, status, duration_ms, host, user, repo_root, repo_branch \
     FROM history \
     WHERE command_text COLLATE NOCASE LIKE ?1 ESCAPE '\\' AND start_cwd = ?2 \
     ORDER BY completed_at DESC, event_sequence DESC LIMIT ?3";

enum QueueMessage {
    Write(Box<HistoryEntry>),
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

    pub fn open_with_context(
        path: &Path,
        queue_capacity: usize,
        context: Box<dyn RepositoryContextProvider>,
    ) -> Result<Self, HistoryError> {
        Self::open_with_limits_and_context(
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
            context,
        )
    }

    pub(crate) fn open_with_limits(
        path: &Path,
        queue_capacity: usize,
        max_rows: u64,
        retention_days: u64,
    ) -> Result<Self, HistoryError> {
        Self::open_with_limits_and_context(
            path,
            queue_capacity,
            max_rows,
            retention_days,
            Box::new(NullRepositoryContextProvider),
        )
    }

    pub(crate) fn open_with_limits_and_context(
        path: &Path,
        queue_capacity: usize,
        max_rows: u64,
        retention_days: u64,
        context: Box<dyn RepositoryContextProvider>,
    ) -> Result<Self, HistoryError> {
        let store_path = path.to_path_buf();
        create_store_dir(&store_path)?;
        let connection = open_connection(&store_path)?;
        let (sender, receiver) = mpsc::sync_channel(queue_capacity);
        let settings = WriterSettings {
            max_rows,
            retention_days,
        };
        let writer = thread::spawn(move || writer_loop(receiver, connection, settings, context));
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

fn commit_partial_batch(
    connection: &rusqlite::Connection,
    pending: &mut usize,
    error_message: &'static str,
) {
    if *pending == 0 {
        return;
    }
    if execute_batch_with_lock_retry(connection, "COMMIT;").is_err() {
        trace_history_failure(&HistoryError::new(HistoryErrorKind::Write, error_message));
        let _ = connection.execute_batch("ROLLBACK;");
    }
    *pending = 0;
}

fn commit_full_batch(
    connection: &rusqlite::Connection,
    pending: &mut usize,
    settings: &WriterSettings,
) {
    if execute_batch_with_lock_retry(connection, "COMMIT;").is_err() {
        trace_history_failure(&HistoryError::new(
            HistoryErrorKind::Write,
            "writer could not commit a batch",
        ));
        let _ = connection.execute_batch("ROLLBACK;");
        *pending = 0;
    } else {
        *pending = 0;
        let _ = prune(connection, settings);
    }
}

fn writer_loop(
    receiver: Receiver<QueueMessage>,
    connection: rusqlite::Connection,
    settings: WriterSettings,
    context: Box<dyn RepositoryContextProvider>,
) {
    let mut pending = 0usize;
    let mut repo_cache = RepoContextCache::default();
    loop {
        let message = if pending == 0 {
            match receiver.recv() {
                Ok(message) => message,
                Err(_) => break,
            }
        } else {
            match receiver.try_recv() {
                Ok(message) => message,
                Err(TryRecvError::Empty) => {
                    commit_partial_batch(
                        &connection,
                        &mut pending,
                        "writer could not commit an idle batch",
                    );
                    continue;
                }
                Err(TryRecvError::Disconnected) => {
                    commit_partial_batch(
                        &connection,
                        &mut pending,
                        "writer could not commit a partial batch",
                    );
                    let _ = prune(&connection, &settings);
                    break;
                }
            }
        };
        match message {
            QueueMessage::Write(entry) => {
                // Enrich outside the SQLite transaction so Git cannot hold the
                // writer lock (M-032). Timeout/absence still insert the row.
                let entry = enrich_repo_context(*entry, context.as_ref(), &mut repo_cache);
                if pending == 0
                    && execute_batch_with_lock_retry_until(
                        &connection,
                        "BEGIN IMMEDIATE;",
                        MIGRATE_BUSY_DEADLINE_MS,
                    )
                    .is_err()
                {
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
                            commit_full_batch(&connection, &mut pending, &settings);
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
                commit_partial_batch(
                    &connection,
                    &mut pending,
                    "writer could not commit a partial batch",
                );
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
        match self.sender.try_send(QueueMessage::Write(Box::new(entry))) {
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

impl HistoryRecorder for std::sync::Arc<QueuedHistoryStore> {
    fn record(&self, entry: HistoryEntry) -> Result<(), HistoryError> {
        (**self).record(entry)
    }
}

impl HistorySearch for QueuedHistoryStore {
    fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        query(
            &connection,
            &format!(
                "SELECT {HISTORY_COLUMNS} \
                 FROM history ORDER BY completed_at DESC, event_sequence DESC LIMIT ?1"
            ),
            &[limit.to_string()],
        )
    }

    fn exact_prefix(&self, prefix: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        let escaped = escape_like(prefix);
        let pattern = format!("{escaped}%");
        if escaped == prefix {
            query(&connection, EXACT_PREFIX_SQL, &[pattern, limit.to_string()])
        } else {
            query(
                &connection,
                EXACT_PREFIX_ESCAPE_SQL,
                &[pattern, limit.to_string()],
            )
        }
    }

    fn exact_prefix_in_cwd(
        &self,
        prefix: &str,
        cwd: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        let escaped = escape_like(prefix);
        let pattern = format!("{escaped}%");
        let params = [pattern, cwd.to_owned(), limit.to_string()];
        if escaped == prefix {
            query(&connection, EXACT_PREFIX_CWD_SQL, &params)
        } else {
            query(&connection, EXACT_PREFIX_CWD_ESCAPE_SQL, &params)
        }
    }

    fn by_cwd(&self, cwd: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        query(
            &connection,
            &format!(
                "SELECT {HISTORY_COLUMNS} \
                 FROM history WHERE start_cwd = ?1 \
                 ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2"
            ),
            &[cwd.to_owned(), limit.to_string()],
        )
    }

    fn by_repo(&self, repo_root: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        query(
            &connection,
            &format!(
                "SELECT {HISTORY_COLUMNS} \
                 FROM history WHERE repo_root = ?1 \
                 ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2"
            ),
            &[repo_root.to_owned(), limit.to_string()],
        )
    }

    fn by_branch(
        &self,
        repo_branch: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        query(
            &connection,
            &format!(
                "SELECT {HISTORY_COLUMNS} \
                 FROM history WHERE repo_branch = ?1 \
                 ORDER BY completed_at DESC, event_sequence DESC LIMIT ?2"
            ),
            &[repo_branch.to_owned(), limit.to_string()],
        )
    }

    fn fuzzy(&self, needle: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        Ok(rank_fuzzy(
            self.recent(crate::history::FUZZY_CANDIDATE_LIMIT)?,
            needle,
            limit,
        ))
    }

    fn fuzzy_in_cwd(
        &self,
        needle: &str,
        cwd: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        Ok(rank_fuzzy(
            self.by_cwd(cwd, crate::history::FUZZY_CANDIDATE_LIMIT)?,
            needle,
            limit,
        ))
    }

    fn failed(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
        query(
            &connection,
            &format!(
                "SELECT {HISTORY_COLUMNS} \
                 FROM history WHERE status != 0 \
                 ORDER BY completed_at DESC, event_sequence DESC LIMIT ?1"
            ),
            &[limit.to_string()],
        )
    }
}

impl HistorySearch for std::sync::Arc<QueuedHistoryStore> {
    fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).recent(limit)
    }

    fn exact_prefix(&self, prefix: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).exact_prefix(prefix, limit)
    }

    fn exact_prefix_in_cwd(
        &self,
        prefix: &str,
        cwd: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).exact_prefix_in_cwd(prefix, cwd, limit)
    }

    fn by_cwd(&self, cwd: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).by_cwd(cwd, limit)
    }

    fn by_repo(&self, repo_root: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).by_repo(repo_root, limit)
    }

    fn by_branch(
        &self,
        repo_branch: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).by_branch(repo_branch, limit)
    }

    fn fuzzy(&self, needle: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).fuzzy(needle, limit)
    }

    fn fuzzy_in_cwd(
        &self,
        needle: &str,
        cwd: &str,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).fuzzy_in_cwd(needle, cwd, limit)
    }

    fn failed(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        (**self).failed(limit)
    }
}

impl HistoryControl for QueuedHistoryStore {
    fn count(&self) -> Result<u64, HistoryError> {
        let connection = open_read_connection(&self.store_path)?;
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
        paths.push(store_sidecar_path(&self.store_path, "-wal"));
        paths.push(store_sidecar_path(&self.store_path, "-shm"));
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

fn store_sidecar_path(store_path: &Path, suffix: &str) -> PathBuf {
    store_path.with_file_name(format!(
        "{}{suffix}",
        store_path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default()
    ))
}

fn tighten_mode(path: &Path, max_mode: u32) -> Result<(), HistoryError> {
    if !path.exists() {
        return Ok(());
    }
    let current = fs::metadata(path)
        .map_err(io_error(HistoryErrorKind::Open))?
        .permissions()
        .mode()
        & 0o777;
    let new_mode = current & max_mode;
    if new_mode != current {
        fs::set_permissions(path, fs::Permissions::from_mode(new_mode))
            .map_err(io_error(HistoryErrorKind::Open))?;
    }
    Ok(())
}

fn apply_created_mode(path: &Path, mode: u32) -> Result<(), HistoryError> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(io_error(HistoryErrorKind::Open))
}

fn restrict_store_permissions(store_path: &Path) -> Result<(), HistoryError> {
    if let Some(dir) = store_path.parent() {
        tighten_mode(dir, DIR_MODE)?;
    }
    tighten_mode(store_path, FILE_MODE)?;
    tighten_mode(&store_sidecar_path(store_path, "-wal"), FILE_MODE)?;
    tighten_mode(&store_sidecar_path(store_path, "-shm"), FILE_MODE)?;
    Ok(())
}

fn create_store_dir(store_path: &Path) -> Result<(), HistoryError> {
    let dir = store_path
        .parent()
        .ok_or_else(|| HistoryError::new(HistoryErrorKind::Open, "store path has no parent"))?;
    if dir.exists() {
        return tighten_mode(dir, DIR_MODE);
    }
    fs::create_dir_all(dir).map_err(io_error(HistoryErrorKind::Open))?;
    apply_created_mode(dir, DIR_MODE)
}

fn open_connection(store_path: &Path) -> Result<rusqlite::Connection, HistoryError> {
    let deadline = std::time::Instant::now() + Duration::from_millis(MIGRATE_BUSY_DEADLINE_MS);
    loop {
        match try_open_connection(store_path) {
            Ok(connection) => return Ok(connection),
            Err(error)
                if is_history_lock_contention(&error) && std::time::Instant::now() < deadline =>
            {
                thread::yield_now();
            }
            Err(error) => return Err(error),
        }
    }
}

fn try_open_connection(store_path: &Path) -> Result<rusqlite::Connection, HistoryError> {
    let created = !store_path.exists();
    let connection =
        rusqlite::Connection::open(store_path).map_err(history_error(HistoryErrorKind::Open))?;
    if created {
        apply_created_mode(store_path, FILE_MODE)?;
    }
    connection
        .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))
        .map_err(history_error(HistoryErrorKind::Open))?;
    ensure_wal_mode(&connection)?;
    migrate(&connection)?;
    restrict_store_permissions(store_path)?;
    Ok(connection)
}

fn open_read_connection(store_path: &Path) -> Result<rusqlite::Connection, HistoryError> {
    if !store_path.exists() {
        return Err(HistoryError::new(
            HistoryErrorKind::Open,
            "store does not exist",
        ));
    }
    let connection =
        rusqlite::Connection::open_with_flags(store_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(history_error(HistoryErrorKind::Open))?;
    connection
        .busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))
        .map_err(history_error(HistoryErrorKind::Open))?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(history_error(HistoryErrorKind::Read))?;
    if version < SCHEMA_VERSION {
        return Err(HistoryError::new(
            HistoryErrorKind::Read,
            "store schema is not ready",
        ));
    }
    Ok(connection)
}

fn ensure_wal_mode(connection: &rusqlite::Connection) -> Result<(), HistoryError> {
    let mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(history_error(HistoryErrorKind::Open))?;
    if mode.eq_ignore_ascii_case("wal") {
        return Ok(());
    }
    connection
        .execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(history_error(HistoryErrorKind::Open))?;
    Ok(())
}

fn migrate(connection: &rusqlite::Connection) -> Result<(), HistoryError> {
    let deadline = std::time::Instant::now() + Duration::from_millis(MIGRATE_BUSY_DEADLINE_MS);
    loop {
        match try_migrate(connection) {
            Ok(()) => return Ok(()),
            Err(error) if is_sqlite_lock_contention(&error) => {
                if std::time::Instant::now() >= deadline {
                    return Err(history_error(HistoryErrorKind::Migrate)(error));
                }
                if schema_version(connection).unwrap_or(0) >= SCHEMA_VERSION {
                    return Ok(());
                }
                thread::yield_now();
            }
            Err(error) => return Err(history_error(HistoryErrorKind::Migrate)(error)),
        }
    }
}

fn schema_version(connection: &rusqlite::Connection) -> Result<i64, rusqlite::Error> {
    connection.query_row("PRAGMA user_version", [], |row| row.get(0))
}

#[cfg(test)]
pub(crate) fn apply_schema_v1(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(SCHEMA_V1)
}

fn try_migrate(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    if schema_version(connection)? >= SCHEMA_VERSION {
        return Ok(());
    }
    connection.execute_batch("BEGIN IMMEDIATE;")?;
    let migrated = (|| -> Result<(), rusqlite::Error> {
        let version = schema_version(connection)?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        if version < 1 {
            connection.execute_batch(SCHEMA_V1)?;
        }
        if version < 2 {
            connection.execute_batch(SCHEMA_V2_INDEX)?;
        }
        if version < 3 {
            connection.execute_batch(SCHEMA_V3_COLUMNS)?;
        }
        connection.execute(&format!("PRAGMA user_version = {SCHEMA_VERSION}"), [])?;
        Ok(())
    })();
    if migrated.is_err() {
        let _ = connection.execute_batch("ROLLBACK;");
        return migrated;
    }
    connection.execute_batch("COMMIT;")?;
    Ok(())
}

fn is_sqlite_lock_contention(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
}

fn is_history_lock_contention(error: &HistoryError) -> bool {
    matches!(
        error.kind(),
        HistoryErrorKind::Open | HistoryErrorKind::Migrate
    ) && (error.to_string().contains("database is locked")
        || error.to_string().contains("database is busy"))
}

fn execute_batch_with_lock_retry(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<(), rusqlite::Error> {
    execute_batch_with_lock_retry_until(connection, sql, BUSY_TIMEOUT_MS)
}

fn execute_batch_with_lock_retry_until(
    connection: &rusqlite::Connection,
    sql: &str,
    timeout_ms: u64,
) -> Result<(), rusqlite::Error> {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match connection.execute_batch(sql) {
            Ok(()) => return Ok(()),
            Err(error)
                if is_sqlite_lock_contention(&error) && std::time::Instant::now() < deadline =>
            {
                thread::yield_now();
            }
            Err(error) => return Err(error),
        }
    }
}

fn insert(connection: &rusqlite::Connection, entry: &HistoryEntry) -> Result<(), HistoryError> {
    connection
        .execute(
            "INSERT OR IGNORE INTO history \
             (session_id, event_sequence, history_number, command_text, start_cwd, \
              completed_at, status, duration_ms, host, user, repo_root, repo_branch) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                entry.repo_root,
                entry.repo_branch,
            ],
        )
        .map_err(history_error(HistoryErrorKind::Write))?;
    Ok(())
}

#[derive(Default)]
struct RepoContextCache {
    entries: HashMap<String, (Instant, Option<RepositoryContext>)>,
}

impl RepoContextCache {
    fn get(&self, cwd: &str) -> Option<Option<RepositoryContext>> {
        let (recorded_at, value) = self.entries.get(cwd)?;
        if recorded_at.elapsed() < REPO_CONTEXT_CACHE_TTL {
            Some(value.clone())
        } else {
            None
        }
    }

    fn insert(&mut self, cwd: String, value: Option<RepositoryContext>) {
        if REPO_CONTEXT_CACHE_CAPACITY == 0 {
            return;
        }
        if !self.entries.contains_key(&cwd) && self.entries.len() >= REPO_CONTEXT_CACHE_CAPACITY {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (recorded_at, _))| *recorded_at)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(cwd, (Instant::now(), value));
    }
}

fn enrich_repo_context(
    mut entry: HistoryEntry,
    provider: &dyn RepositoryContextProvider,
    cache: &mut RepoContextCache,
) -> HistoryEntry {
    if entry.repo_root.is_some() || entry.repo_branch.is_some() {
        return sanitize_entry_repo_fields(entry);
    }
    if !Path::new(&entry.start_cwd).is_absolute() {
        return entry;
    }
    let resolved = if let Some(cached) = cache.get(&entry.start_cwd) {
        cached
    } else {
        let looked_up = provider
            .context(Path::new(&entry.start_cwd))
            .unwrap_or_default();
        cache.insert(entry.start_cwd.clone(), looked_up.clone());
        looked_up
    };
    if let Some(context) = resolved {
        entry.repo_root = sanitize_repo_root(&context.root);
        entry.repo_branch = context.branch.as_deref().and_then(sanitize_repo_branch);
        if entry.repo_root.is_none() {
            entry.repo_branch = None;
        }
    }
    entry
}

fn sanitize_entry_repo_fields(mut entry: HistoryEntry) -> HistoryEntry {
    entry.repo_root = entry.repo_root.as_deref().and_then(sanitize_repo_root);
    entry.repo_branch = entry.repo_branch.as_deref().and_then(sanitize_repo_branch);
    if entry.repo_root.is_none() {
        entry.repo_branch = None;
    }
    entry
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

fn rank_fuzzy(mut pool: Vec<HistoryEntry>, needle: &str, limit: usize) -> Vec<HistoryEntry> {
    pool.retain(|entry| crate::history::fuzzy_score(needle, &entry.command_text) > 0);
    pool.sort_by(|left, right| {
        let left_score = crate::history::fuzzy_score(needle, &left.command_text);
        let right_score = crate::history::fuzzy_score(needle, &right.command_text);
        right_score
            .cmp(&left_score)
            .then_with(|| right.completed_at.cmp(&left.completed_at))
            .then_with(|| right.event_sequence.cmp(&left.event_sequence))
    });
    pool.truncate(limit);
    pool
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
                repo_root: row.get(10)?,
                repo_branch: row.get(11)?,
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
    use crate::history::{
        HistoryControl, HistoryError, HistoryErrorKind, HistoryRecorder, HistorySearch,
    };
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;

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
            repo_root: None,
            repo_branch: None,
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

    fn count_rows(path: &Path) -> u64 {
        let store = QueuedHistoryStore::open(path, 32).unwrap();
        store.count().unwrap()
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

    fn assert_unique_keys(path: &Path) {
        let connection = rusqlite::Connection::open(path).unwrap();
        let duplicates: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM (
                    SELECT session_id, event_sequence FROM history
                    GROUP BY session_id, event_sequence HAVING COUNT(*) > 1
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(duplicates, 0, "duplicate (session_id, event_sequence) keys");
    }

    #[test]
    fn concurrent_sessions_write_distinct_rows_without_duplicates() {
        let (dir, path) = temp_store("c1");
        let path = Arc::new(path);
        let handles: Vec<_> = (0..8)
            .map(|session| {
                let path = Arc::clone(&path);
                thread::spawn(move || {
                    let store = QueuedHistoryStore::open(&path, 64).unwrap();
                    let session_id = format!("s{session}");
                    for sequence in 0..32 {
                        enqueue(
                            &store,
                            entry(
                                &session_id,
                                sequence,
                                &format!("cmd {sequence}"),
                                "/w",
                                "2026-08-15T10:00:00Z",
                            ),
                        );
                    }
                    drop(store);
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert_unique_keys(&path);
        assert_eq!(count_rows(&path), 256);
        let connection = rusqlite::Connection::open(path.as_path()).unwrap();
        for session in 0..8 {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM history WHERE session_id = ?1",
                    [format!("s{session}")],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 32);
        }
        drop(dir);
    }

    #[test]
    fn concurrent_distinct_sessions_both_land() {
        let (dir, path) = temp_store("c2");
        let path = Arc::new(path);
        let alpha = {
            let path = Arc::clone(&path);
            thread::spawn(move || {
                let store = QueuedHistoryStore::open(&path, 32).unwrap();
                for sequence in 0..16 {
                    enqueue(
                        &store,
                        entry("alpha", sequence, "cmd", "/w", "2026-08-15T10:00:00Z"),
                    );
                }
                drop(store);
            })
        };
        let beta = {
            let path = Arc::clone(&path);
            thread::spawn(move || {
                let store = QueuedHistoryStore::open(&path, 32).unwrap();
                for sequence in 0..16 {
                    enqueue(
                        &store,
                        entry("beta", sequence, "cmd", "/w", "2026-08-15T10:00:01Z"),
                    );
                }
                drop(store);
            })
        };
        alpha.join().unwrap();
        beta.join().unwrap();
        assert_unique_keys(&path);
        assert_eq!(count_rows(&path), 32);
        let connection = rusqlite::Connection::open(path.as_path()).unwrap();
        let alpha_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM history WHERE session_id = 'alpha'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let beta_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM history WHERE session_id = 'beta'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(alpha_count, 16);
        assert_eq!(beta_count, 16);
        drop(dir);
    }

    #[test]
    fn reader_queries_while_writer_inserts_and_prunes() {
        let (dir, path) = temp_store("c3");
        let path = Arc::new(path);
        let reader_path = Arc::clone(&path);
        let (writer_ready_tx, writer_ready_rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            writer_ready_rx
                .recv()
                .expect("writer must open before reader");
            let store = QueuedHistoryStore::open(&reader_path, 32).unwrap();
            for _ in 0..200 {
                let _ = store.count();
                let _ = store.recent(10);
                let _ = store.exact_prefix("cmd", 5);
                let _ = store.by_cwd("/w", 5);
                let _ = store.by_repo("/w", 5);
                let _ = store.by_branch("main", 5);
                let _ = store.failed(5);
                thread::yield_now();
            }
        });
        {
            let store = QueuedHistoryStore::open_with_limits(&path, 64, 40, 36_500).unwrap();
            writer_ready_tx.send(()).expect("reader must be waiting");
            for sequence in 0..64 {
                enqueue(
                    &store,
                    entry(
                        "s1",
                        sequence,
                        &format!("cmd {sequence}"),
                        "/w",
                        &format!("2026-08-15T10:{sequence:02}:00Z"),
                    ),
                );
            }
            drop(store);
        }
        reader.join().unwrap();
        assert_unique_keys(&path);
        let count = count_rows(&path);
        assert!(
            count <= 40,
            "retention cap must prune to 40 rows; got {count}"
        );
        let connection = rusqlite::Connection::open(path.as_path()).unwrap();
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
    fn concurrent_replay_of_same_key_is_idempotent() {
        let (dir, path) = temp_store("c4");
        let first = entry("s1", 1, "echo one", "/work", "2026-08-15T10:00:00Z");
        {
            let store = QueuedHistoryStore::open(&path, 8).unwrap();
            store.record(first.clone()).unwrap();
        }
        {
            let store = QueuedHistoryStore::open(&path, 8).unwrap();
            store.record(first).unwrap();
        }
        assert_eq!(count_rows(&path), 1);
        assert_unique_keys(&path);
        drop(dir);
    }

    #[test]
    fn full_queue_returns_quickly_and_drains_without_hang() {
        let (dir, path) = temp_store("c6");
        let mut saw_queue_full = false;
        {
            let store = QueuedHistoryStore::open(&path, 2).unwrap();
            for sequence in 0..8 {
                match store.record(entry("s1", sequence, "cmd", "/w", "2026-08-15T10:00:00Z")) {
                    Ok(()) => {}
                    Err(error) if error.kind() == HistoryErrorKind::QueueFull => {
                        saw_queue_full = true;
                    }
                    Err(error) => panic!("unexpected record error: {error}"),
                }
            }
            drop(store);
        }
        assert!(
            saw_queue_full,
            "queue capacity 2 must reject at least one record"
        );
        let count = count_rows(&path);
        assert!(count <= 8, "at most eight records may commit; got {count}");
        assert!(count >= 1, "at least one record must drain");
        assert_unique_keys(&path);
        drop(dir);
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
        assert_eq!(index_count(&connection, "history_prefix"), 1);
        assert_eq!(index_count(&connection, "history_prefix_completed"), 1);
        assert_eq!(index_count(&connection, "history_repo_root"), 1);
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
        let prefix_cwd = store.exact_prefix_in_cwd("git", "/repo", 10).unwrap();
        assert_eq!(prefix_cwd.len(), 2);
        assert!(
            store
                .exact_prefix_in_cwd("git", "/home", 10)
                .unwrap()
                .is_empty()
        );
        let fuzzy_home = store.fuzzy_in_cwd("ls", "/home", 10).unwrap();
        assert_eq!(fuzzy_home.len(), 1);
        assert_eq!(fuzzy_home[0].command_text, "ls -la");
        assert!(store.fuzzy_in_cwd("ls", "/repo", 10).unwrap().is_empty());
        drop(dir);
    }

    #[test]
    fn fuzzy_ranks_prefix_above_subsequence_over_bounded_pool() {
        let (dir, path) = temp_store("fuzzy");
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            store
                .record(entry("s1", 1, "ls", "/w", "2026-08-15T10:00:00Z"))
                .unwrap();
            store
                .record(entry("s1", 2, "git status", "/w", "2026-08-15T10:00:01Z"))
                .unwrap();
            store
                .record(entry("s1", 3, "git stash", "/w", "2026-08-15T10:00:02Z"))
                .unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        let rows = store.fuzzy("git sta", 10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].command_text, "git stash");
        assert_eq!(rows[1].command_text, "git status");
        assert!(
            store
                .fuzzy("ls", 10)
                .unwrap()
                .iter()
                .any(|row| row.command_text == "ls")
        );
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

    const WAL_SENTINEL: &str = "secret-wal-token";

    fn sidecar(store: &Path, suffix: &str) -> PathBuf {
        store.with_file_name(format!(
            "{}{suffix}",
            store
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default()
        ))
    }

    fn raw_history_insert(
        connection: &rusqlite::Connection,
        sequence: u64,
        command: &str,
        completed_at: &str,
    ) {
        connection
            .execute(
                "INSERT OR IGNORE INTO history \
                 (session_id, event_sequence, history_number, command_text, start_cwd, \
                  completed_at, status, duration_ms, host, user) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    "s1",
                    sequence,
                    sequence as i64,
                    command,
                    "/w",
                    completed_at,
                    0i32,
                    None::<u64>,
                    "host",
                    "user",
                ],
            )
            .unwrap();
    }

    fn assert_closed_history_error(error: &HistoryError) {
        assert!(
            matches!(
                error.kind(),
                HistoryErrorKind::Open
                    | HistoryErrorKind::Migrate
                    | HistoryErrorKind::StorageFailure
            ),
            "unexpected kind: {error}"
        );
        let shown = error.to_string();
        assert!(!shown.contains(WAL_SENTINEL), "{shown}");
        let diagnostic = history_failure_diagnostic(error);
        assert_eq!(
            diagnostic,
            format!("event=history_storage_error kind={}", error.kind().as_str())
        );
        assert!(!diagnostic.contains(WAL_SENTINEL), "{diagnostic}");
    }

    fn assert_main_store_not_destroyed(path: &Path) {
        assert!(path.exists(), "must not unlink the db");
        assert_ne!(
            std::fs::metadata(path).unwrap().len(),
            0,
            "must not replace the store with an empty file"
        );
    }

    fn commit_sentinel_row(path: &Path) {
        let store = QueuedHistoryStore::open(path, 8).unwrap();
        enqueue(
            &store,
            entry("s1", 1, WAL_SENTINEL, "/w", "2026-08-16T10:00:00Z"),
        );
        drop(store);
        assert_eq!(count_rows(path), 1);
    }

    fn ensure_wal_sidecar_exists(path: &Path) {
        {
            let connection = rusqlite::Connection::open(path).unwrap();
            connection
                .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_checkpoint(TRUNCATE);")
                .unwrap();
        }
        let wal = sidecar(path, "-wal");
        if wal.exists() {
            return;
        }
        let connection = rusqlite::Connection::open(path).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; BEGIN IMMEDIATE;")
            .unwrap();
        raw_history_insert(&connection, 1, WAL_SENTINEL, "2026-08-16T10:00:00Z");
        connection.execute_batch("COMMIT;").unwrap();
        drop(connection);
        if !wal.exists() {
            std::fs::write(&wal, b"").unwrap();
        }
    }

    fn assert_corrupt_sidecar_is_safe(path: &Path, sidecar_path: &Path) {
        std::fs::write(sidecar_path, vec![0xFF; 4096]).unwrap();
        match QueuedHistoryStore::open(path, 8) {
            Ok(store) => {
                assert_eq!(store.count().unwrap(), 1);
                assert_eq!(store.recent(1).unwrap()[0].command_text, WAL_SENTINEL);
                drop(store);
            }
            Err(error) => {
                assert_closed_history_error(&error);
                assert_main_store_not_destroyed(path);
                let _ = std::fs::remove_file(sidecar_path);
                let store = QueuedHistoryStore::open(path, 8).unwrap();
                assert_eq!(store.count().unwrap(), 1);
                drop(store);
            }
        }
        assert_main_store_not_destroyed(path);
        assert_unique_keys(path);
    }

    #[test]
    fn crash_mid_transaction_rolls_back_and_retry_is_idempotent() {
        let (dir, path) = temp_store("k1");
        commit_sentinel_row(&path);
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE;")
                .unwrap();
            raw_history_insert(&connection, 2, "echo uncommitted", "2026-08-16T10:00:01Z");
            drop(connection);
        }
        {
            let store = QueuedHistoryStore::open(&path, 8).unwrap();
            assert_eq!(store.count().unwrap(), 1);
            assert_eq!(store.recent(1).unwrap()[0].command_text, WAL_SENTINEL);
            enqueue(
                &store,
                entry("s1", 1, WAL_SENTINEL, "/w", "2026-08-16T10:00:00Z"),
            );
            assert_eq!(store.count().unwrap(), 1);
            enqueue(
                &store,
                entry(
                    "s1",
                    2,
                    "echo committed-after-crash",
                    "/w",
                    "2026-08-16T10:00:02Z",
                ),
            );
        }
        assert_eq!(count_rows(&path), 2);
        assert_unique_keys(&path);
        drop(dir);
    }

    #[test]
    fn corrupt_wal_does_not_destroy_committed_store() {
        let (dir, path) = temp_store("k2");
        commit_sentinel_row(&path);
        ensure_wal_sidecar_exists(&path);
        assert_corrupt_sidecar_is_safe(&path, &sidecar(&path, "-wal"));
        drop(dir);
    }

    #[test]
    fn corrupt_shm_does_not_destroy_committed_store() {
        let (dir, path) = temp_store("k3");
        commit_sentinel_row(&path);
        ensure_wal_sidecar_exists(&path);
        let shm = sidecar(&path, "-shm");
        if !shm.exists() {
            std::fs::write(&shm, b"").unwrap();
        }
        assert_corrupt_sidecar_is_safe(&path, &shm);
        drop(dir);
    }

    #[test]
    fn corrupt_main_db_fails_closed_without_replacing_the_file() {
        let (dir, path) = temp_store("k4");
        commit_sentinel_row(&path);
        let original_len = std::fs::metadata(&path).unwrap().len();
        assert!(original_len > 0);
        std::fs::write(&path, vec![0xFF; 4096]).unwrap();
        let corrupted_len = std::fs::metadata(&path).unwrap().len();
        assert_eq!(corrupted_len, 4096);
        let result = QueuedHistoryStore::open(&path, 8);
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("corrupt main db must not open as a fresh store"),
        };
        assert_closed_history_error(&error);
        assert_main_store_not_destroyed(&path);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), corrupted_len as usize);
        assert!(
            bytes.iter().all(|byte| *byte == 0xFF),
            "corrupt main db must not be replaced with a new sqlite file"
        );
        drop(dir);
    }

    const PERM_SENTINEL: &str = "secret-perm-token";

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn chmod(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    fn commit_perm_sentinel(path: &Path) {
        let store = QueuedHistoryStore::open(path, 8).unwrap();
        enqueue(
            &store,
            entry("s1", 1, PERM_SENTINEL, "/w", "2026-08-16T12:00:00Z"),
        );
        drop(store);
    }

    fn assert_closed_perm_error(error: &HistoryError) {
        assert!(
            matches!(
                error.kind(),
                HistoryErrorKind::Open
                    | HistoryErrorKind::Migrate
                    | HistoryErrorKind::StorageFailure
            ),
            "unexpected kind: {error}"
        );
        let shown = error.to_string();
        assert!(!shown.contains(PERM_SENTINEL), "{shown}");
        let diagnostic = history_failure_diagnostic(error);
        assert_eq!(
            diagnostic,
            format!("event=history_storage_error kind={}", error.kind().as_str())
        );
        assert!(!diagnostic.contains(PERM_SENTINEL), "{diagnostic}");
    }

    fn assert_foreign_probe_is_nobody() {
        let output = Command::new("sudo")
            .args(["-n", "-u", "nobody", "id", "-u"])
            .output()
            .expect("sudo must spawn for foreign-user evidence");
        assert!(
            output.status.success(),
            "sudo -n -u nobody must work on this host: status={:?} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(
            uid, "65534",
            "foreign probe must use uid 65534(nobody), not the owner uid"
        );
    }

    fn foreign_probe_output(args: &[&str]) -> std::process::Output {
        let mut command = Command::new("sudo");
        command.arg("-n").arg("-u").arg("nobody").arg("--");
        command.args(args);
        command.output().expect("foreign probe must spawn")
    }

    fn assert_foreign_read_denied(path: &Path) {
        let input = format!("if={}", path.display());
        let output = foreign_probe_output(&["dd", &input, "of=/dev/null", "bs=1", "count=1"]);
        assert!(
            !output.status.success(),
            "foreign user must not read {}",
            path.display()
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !combined.contains(PERM_SENTINEL),
            "foreign read must not expose store payload: {combined}"
        );
    }

    fn assert_foreign_directory_denied(path: &Path) {
        let path_arg = path.to_string_lossy();
        let output = foreign_probe_output(&["ls", &path_arg]);
        assert!(
            !output.status.success(),
            "foreign user must not list {}",
            path.display()
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !combined.contains(PERM_SENTINEL),
            "foreign directory probe must not expose store payload: {combined}"
        );
    }

    fn wait_for_owner_count(store: &QueuedHistoryStore, expected: u64) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if store.count().unwrap() == expected {
                return;
            }
            if std::time::Instant::now() > deadline {
                panic!("owner could not read count {expected} from the store");
            }
            thread::yield_now();
        }
    }

    #[test]
    fn foreign_user_cannot_open_store_paths() {
        assert_foreign_probe_is_nobody();

        let (dir, path) = temp_store("foreign");
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        enqueue(
            &store,
            entry("s1", 1, PERM_SENTINEL, "/w", "2026-08-16T12:00:00Z"),
        );
        wait_for_owner_count(&store, 1);

        assert_eq!(mode_of(dir.path()), 0o700);
        assert_eq!(mode_of(&path), 0o600);

        assert_foreign_directory_denied(dir.path());
        assert_foreign_read_denied(&path);

        let wal = sidecar(&path, "-wal");
        let shm = sidecar(&path, "-shm");
        assert!(
            wal.exists(),
            "WAL sidecar must exist while the writer is live"
        );
        assert!(
            shm.exists(),
            "SHM sidecar must exist while the writer is live"
        );
        assert_eq!(mode_of(&wal), 0o600);
        assert_eq!(mode_of(&shm), 0o600);
        assert_foreign_read_denied(&wal);
        assert_foreign_read_denied(&shm);

        drop(store);
        drop(dir);
    }

    #[test]
    fn wal_and_shm_files_are_user_only() {
        let (dir, path) = temp_store("p1");
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        enqueue(
            &store,
            entry("s1", 1, PERM_SENTINEL, "/w", "2026-08-16T12:00:00Z"),
        );
        assert_eq!(mode_of(dir.path()), 0o700);
        assert_eq!(mode_of(&path), 0o600);
        let wal = sidecar(&path, "-wal");
        let shm = sidecar(&path, "-shm");
        assert!(
            wal.exists(),
            "WAL sidecar must exist while the writer is live"
        );
        assert!(
            shm.exists(),
            "SHM sidecar must exist while the writer is live"
        );
        assert_eq!(mode_of(&wal), 0o600);
        assert_eq!(mode_of(&shm), 0o600);
        drop(store);
        drop(dir);
    }

    #[test]
    fn world_accessible_store_is_tightened() {
        let (dir, path) = temp_store("p2");
        {
            let store = QueuedHistoryStore::open(&path, 8).unwrap();
            enqueue(
                &store,
                entry("s1", 1, PERM_SENTINEL, "/w", "2026-08-16T12:00:00Z"),
            );
            let wal = sidecar(&path, "-wal");
            let shm = sidecar(&path, "-shm");
            assert!(
                wal.exists(),
                "WAL sidecar must exist while the writer is live"
            );
            assert!(
                shm.exists(),
                "SHM sidecar must exist while the writer is live"
            );
            chmod(dir.path(), 0o777);
            chmod(&path, 0o644);
            chmod(&wal, 0o666);
            chmod(&shm, 0o666);
            drop(store);
        }
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        assert_eq!(mode_of(dir.path()), 0o700);
        assert_eq!(mode_of(&path), 0o600);
        let wal = sidecar(&path, "-wal");
        let shm = sidecar(&path, "-shm");
        assert!(
            wal.exists(),
            "WAL sidecar must exist while the writer is live"
        );
        assert!(
            shm.exists(),
            "SHM sidecar must exist while the writer is live"
        );
        assert_eq!(mode_of(&wal), 0o600);
        assert_eq!(mode_of(&shm), 0o600);
        assert_eq!(store.count().unwrap(), 1);
        assert_eq!(store.recent(1).unwrap()[0].command_text, PERM_SENTINEL);
        drop(store);
        drop(dir);
    }

    #[test]
    fn restrictive_file_is_not_made_more_permissive() {
        let (dir, path) = temp_store("p3");
        commit_perm_sentinel(&path);
        chmod(&path, 0o400);
        match QueuedHistoryStore::open(&path, 8) {
            Ok(store) => {
                assert_eq!(mode_of(&path), 0o400);
                assert_eq!(store.count().unwrap(), 1);
                drop(store);
            }
            Err(error) => {
                assert_closed_perm_error(&error);
                assert_eq!(mode_of(&path), 0o400);
                assert!(path.exists());
                assert_ne!(std::fs::metadata(&path).unwrap().len(), 0);
            }
        }
        drop(dir);
    }

    #[test]
    fn unreadable_store_fails_closed_without_widening() {
        let (dir, path) = temp_store("p4");
        commit_perm_sentinel(&path);
        let original_len = std::fs::metadata(&path).unwrap().len();
        chmod(&path, 0o000);
        let error = match QueuedHistoryStore::open(&path, 8) {
            Err(error) => error,
            Ok(_) => panic!("mode 0000 store must not open"),
        };
        assert_closed_perm_error(&error);
        assert_eq!(mode_of(&path), 0o000);
        assert!(path.exists());
        assert_eq!(std::fs::metadata(&path).unwrap().len(), original_len);
        drop(dir);
    }

    #[test]
    fn schema_v1_store_migrates_to_v2_prefix_index() {
        let (dir, path) = temp_store("qa");
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute_batch("PRAGMA journal_mode=WAL;")
                .unwrap();
            connection.execute_batch(SCHEMA_V1).unwrap();
            connection.execute("PRAGMA user_version = 1", []).unwrap();
            connection
                .execute(
                    "INSERT OR IGNORE INTO history \
                     (session_id, event_sequence, history_number, command_text, start_cwd, \
                      completed_at, status, duration_ms, host, user) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        "s1",
                        1u64,
                        1i64,
                        "git status",
                        "/w",
                        "2026-08-16T14:00:00Z",
                        0i32,
                        None::<u64>,
                        "host",
                        "user",
                    ],
                )
                .unwrap();
        }
        let original_len = std::fs::metadata(&path).unwrap().len();
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        drop(store);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index_count(&connection, "history_prefix"), 1);
        assert_eq!(index_count(&connection, "history_prefix_completed"), 1);
        assert_eq!(index_count(&connection, "history_repo_root"), 1);
        assert_eq!(count_rows(&path), 1);
        assert_eq!(
            QueuedHistoryStore::open(&path, 8)
                .unwrap()
                .recent(1)
                .unwrap()[0]
                .command_text,
            "git status"
        );
        assert_ne!(std::fs::metadata(&path).unwrap().len(), 0);
        assert!(std::fs::metadata(&path).unwrap().len() >= original_len);
        drop(dir);
    }

    #[test]
    fn empty_store_opens_at_schema_v2() {
        let (dir, path) = temp_store("qb");
        {
            let store = QueuedHistoryStore::open(&path, 8).unwrap();
            drop(store);
        }
        let connection = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index_count(&connection, "history_prefix"), 1);
        assert_eq!(index_count(&connection, "history_prefix_completed"), 1);
        assert_eq!(index_count(&connection, "history_repo_root"), 1);
        drop(dir);
    }

    #[test]
    fn schema_v2_store_migrates_to_v3_repo_columns() {
        let (dir, path) = temp_store("v2v3");
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute_batch("PRAGMA journal_mode=WAL;")
                .unwrap();
            connection.execute_batch(SCHEMA_V1).unwrap();
            connection.execute_batch(SCHEMA_V2_INDEX).unwrap();
            connection.execute("PRAGMA user_version = 2", []).unwrap();
            connection
                .execute(
                    "INSERT OR IGNORE INTO history \
                     (session_id, event_sequence, history_number, command_text, start_cwd, \
                      completed_at, status, duration_ms, host, user) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        "s1",
                        1u64,
                        1i64,
                        "git status",
                        "/w",
                        "2026-08-16T16:00:00Z",
                        0i32,
                        None::<u64>,
                        "host",
                        "user",
                    ],
                )
                .unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        drop(store);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert_eq!(index_count(&connection, "history_repo_root"), 1);
        let migrated = QueuedHistoryStore::open(&path, 8)
            .unwrap()
            .recent(1)
            .unwrap();
        assert_eq!(migrated[0].command_text, "git status");
        assert_eq!(migrated[0].repo_root, None);
        assert_eq!(migrated[0].repo_branch, None);
        drop(dir);
    }

    struct StaticContextProvider {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        result: Result<Option<RepositoryContext>, crate::provider::ProviderError>,
    }

    impl RepositoryContextProvider for StaticContextProvider {
        fn context(
            &self,
            _cwd: &Path,
        ) -> Result<Option<RepositoryContext>, crate::provider::ProviderError> {
            self.calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.result.clone()
        }
    }

    #[test]
    fn writer_enriches_from_context_provider_and_caches_cwd() {
        let (dir, path) = temp_store("enrich");
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        {
            let store = QueuedHistoryStore::open_with_context(
                &path,
                8,
                Box::new(StaticContextProvider {
                    calls: std::sync::Arc::clone(&calls),
                    result: Ok(Some(RepositoryContext {
                        root: "/repo/root".to_owned(),
                        branch: Some("hist-branch".to_owned()),
                    })),
                }),
            )
            .unwrap();
            enqueue(
                &store,
                entry("s1", 1, "echo one", "/work", "2026-08-16T16:00:00Z"),
            );
            enqueue(
                &store,
                entry("s1", 2, "echo two", "/work", "2026-08-16T16:00:01Z"),
            );
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        let rows = store.recent(2).unwrap();
        assert_eq!(rows[0].command_text, "echo two");
        assert_eq!(rows[0].repo_root.as_deref(), Some("/repo/root"));
        assert_eq!(rows[0].repo_branch.as_deref(), Some("hist-branch"));
        assert_eq!(rows[1].repo_root.as_deref(), Some("/repo/root"));
        let by_repo = store.by_repo("/repo/root", 10).unwrap();
        assert_eq!(by_repo.len(), 2);
        let by_branch = store.by_branch("hist-branch", 10).unwrap();
        assert_eq!(by_branch.len(), 2);
        drop(dir);
    }

    #[test]
    fn by_branch_matches_exact_name_newest_first() {
        let (dir, path) = temp_store("by-branch");
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            let mut main_old = entry("s1", 1, "echo main-old", "/w", "2026-08-16T16:00:00Z");
            main_old.repo_root = Some("/repo/root".to_owned());
            main_old.repo_branch = Some("main".to_owned());
            store.record(main_old).unwrap();
            let mut hist_old = entry("s1", 2, "echo hist-old", "/w", "2026-08-16T16:00:01Z");
            hist_old.repo_root = Some("/repo/root".to_owned());
            hist_old.repo_branch = Some("hist-branch".to_owned());
            store.record(hist_old).unwrap();
            let mut hist_new = entry("s1", 3, "echo hist-new", "/w", "2026-08-16T16:00:02Z");
            hist_new.repo_root = Some("/other/root".to_owned());
            hist_new.repo_branch = Some("hist-branch".to_owned());
            store.record(hist_new).unwrap();
            store
                .record(entry("s1", 4, "echo none", "/w", "2026-08-16T16:00:03Z"))
                .unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        let rows = store.by_branch("hist-branch", 10).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row.command_text.as_str())
                .collect::<Vec<_>>(),
            ["echo hist-new", "echo hist-old"]
        );
        let limited = store.by_branch("hist-branch", 1).unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].command_text, "echo hist-new");
        assert!(store.by_branch("missing", 10).unwrap().is_empty());
        drop(dir);
    }

    #[test]
    fn failed_returns_nonzero_status_newest_first() {
        let (dir, path) = temp_store("failed");
        {
            let store = QueuedHistoryStore::open(&path, 32).unwrap();
            let mut first = entry("s1", 1, "false", "/w", "2026-08-15T10:00:00Z");
            first.status = 1;
            store.record(first).unwrap();
            store
                .record(entry("s1", 2, "true", "/w", "2026-08-15T10:00:01Z"))
                .unwrap();
            let mut third = entry("s1", 3, "exit 2", "/w", "2026-08-15T10:00:02Z");
            third.status = 2;
            store.record(third).unwrap();
            let mut fourth = entry("s1", 4, "old-fail", "/w", "2026-08-15T09:59:00Z");
            fourth.status = 127;
            store.record(fourth).unwrap();
        }
        let store = QueuedHistoryStore::open(&path, 32).unwrap();
        let rows = store.failed(10).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row.command_text.as_str())
                .collect::<Vec<_>>(),
            ["exit 2", "false", "old-fail"]
        );
        assert!(rows.iter().all(|row| row.status != 0));
        let limited = store.failed(2).unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].command_text, "exit 2");
        assert_eq!(limited[1].command_text, "false");
        drop(dir);
    }

    #[test]
    fn writer_keeps_the_row_when_context_lookup_fails() {
        let (dir, path) = temp_store("enrich-fail");
        {
            let store = QueuedHistoryStore::open_with_context(
                &path,
                8,
                Box::new(StaticContextProvider {
                    calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                    result: Err(crate::provider::ProviderError::message("timeout")),
                }),
            )
            .unwrap();
            enqueue(
                &store,
                entry("s1", 1, "echo keep", "/work", "2026-08-16T16:00:00Z"),
            );
        }
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        let rows = store.recent(1).unwrap();
        assert_eq!(rows[0].command_text, "echo keep");
        assert_eq!(rows[0].repo_root, None);
        assert_eq!(rows[0].repo_branch, None);
        drop(dir);
    }

    #[test]
    fn writer_preserves_prefilled_repo_fields_without_lookup() {
        let (dir, path) = temp_store("enrich-pref");
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        {
            let store = QueuedHistoryStore::open_with_context(
                &path,
                8,
                Box::new(StaticContextProvider {
                    calls: std::sync::Arc::clone(&calls),
                    result: Ok(Some(RepositoryContext {
                        root: "/ignored".to_owned(),
                        branch: Some("ignored".to_owned()),
                    })),
                }),
            )
            .unwrap();
            let mut prefilled = entry("s1", 1, "echo pref", "/work", "2026-08-16T16:00:00Z");
            prefilled.repo_root = Some("/explicit/root".to_owned());
            prefilled.repo_branch = Some("explicit".to_owned());
            enqueue(&store, prefilled);
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        let rows = store.recent(1).unwrap();
        assert_eq!(rows[0].repo_root.as_deref(), Some("/explicit/root"));
        assert_eq!(rows[0].repo_branch.as_deref(), Some("explicit"));
        drop(dir);
    }

    #[test]
    fn many_match_prefix_uses_covering_index_and_stays_newest_first() {
        let (dir, path) = temp_store("qc");
        {
            let store = QueuedHistoryStore::open(&path, 64).unwrap();
            for sequence in 0..48u64 {
                enqueue(
                    &store,
                    entry(
                        "s1",
                        sequence,
                        &format!("git cmd {sequence}"),
                        "/w",
                        &format!("2026-08-16T14:{sequence:02}:00Z"),
                    ),
                );
            }
            for sequence in 48..64u64 {
                enqueue(
                    &store,
                    entry(
                        "s1",
                        sequence,
                        "echo other",
                        "/w",
                        &format!("2026-08-16T15:{:02}:00Z", sequence - 48),
                    ),
                );
            }
        }
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        let rows = store.exact_prefix("git", 5).unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].command_text, "git cmd 47");
        assert!(rows.iter().all(|row| row.command_text.starts_with("git ")));
        let connection = rusqlite::Connection::open(&path).unwrap();
        let mut statement = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {EXACT_PREFIX_SQL}"))
            .unwrap();
        let plan_rows = statement
            .query_map(["git%", "5"], |row| row.get::<_, String>(3))
            .unwrap();
        let plan = plan_rows
            .map(|row| row.unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            plan.to_ascii_lowercase()
                .contains("history_prefix_completed"),
            "expected covering index in plan: {plan}"
        );
        drop(store);
        drop(dir);
    }

    const IDLE_SENTINEL: &str = "secret-idle-token";

    fn wait_for_external_count(path: &Path, expected: u64) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut reader = None;
        let mut last = 0u64;
        loop {
            if reader.is_none() {
                reader = QueuedHistoryStore::open(path, 8).ok();
            }
            if let Some(store) = reader.as_ref() {
                if let Ok(count) = store.count() {
                    last = count;
                    if count == expected {
                        return;
                    }
                }
            }
            if std::time::Instant::now() >= deadline {
                panic!("external reader never reached count {expected}; last={last}");
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn idle_writer_commits_one_row_for_a_live_external_reader() {
        let (dir, path) = temp_store("v1");
        let writer = QueuedHistoryStore::open(&path, 8).unwrap();
        enqueue(
            &writer,
            entry("s1", 1, IDLE_SENTINEL, "/w", "2026-08-16T16:00:00Z"),
        );
        wait_for_external_count(&path, 1);
        let reader = QueuedHistoryStore::open(&path, 8).unwrap();
        assert_eq!(reader.recent(1).unwrap()[0].command_text, IDLE_SENTINEL);
        drop(reader);
        drop(writer);
        drop(dir);
    }

    #[test]
    fn idle_writer_commits_a_partial_batch_under_writer_batch_size() {
        let (dir, path) = temp_store("v2");
        let writer = QueuedHistoryStore::open(&path, 64).unwrap();
        for sequence in 0..8u64 {
            enqueue(
                &writer,
                entry(
                    "s1",
                    sequence,
                    &format!("cmd {sequence}"),
                    "/w",
                    "2026-08-16T16:00:00Z",
                ),
            );
        }
        wait_for_external_count(&path, 8);
        assert_unique_keys(&path);
        drop(writer);
        drop(dir);
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
