# HIST-007 slice: WAL crash and corrupt-store recovery

Status: `complete` for WAL crash/corrupt cases K-1–K-4 (2026-08-16). Do not mark
`G2` or `HIST-007` complete. Write-ack budget, permission beyond mode bits, and
many-match prefix latency remain.

## Why this slice (do not pick a different leftover)

Remaining `G2` items, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | WAL crash/corrupt (this plan) | Contract exists; storage tests; no PTY; no second user |
| 2 | Foreign-user / permission beyond mode bits | Needs a second Unix uid or an explicit ADR that mode bits are the whole boundary. `store_files_are_user_only` already covers `0700`/`0600` on dir + db |
| 3 | Many-match `git` prefix (~61 ms p95) | Schema/index or query-plan change (`ORDER BY` over every `git*` match). Likely ADR. Not a small test-only slice |
| 4 | Write-ack p95/p99 budget miss | Correctness is recorded. `docs/history-g2-write-ack-plan.md` forbids product-code latency work unless a test proves the prompt waits on SQLite, samples contain command text, or ACK waits for commit |
| 5 | Contention case 8 (hypothetical v2 on 100k rows) | There is no v2. Empty→v1 is already `schema_creates_tables_and_migrations_run_once` |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-019`, `M-023`, `M-024`, `M-029`–`M-032`.
2. Read this file completely. Do not invent extra cases.
3. Read `docs/history-phase3a-contract.md` section **Durability contract
   (HIST-012)** (Crash, Retry, Storage failure).
4. Read `docs/adr/0005-history-storage.md` sections 5, 7, and Validation plan.
5. Read `crates/cli/src/storage.rs`: `QueuedHistoryStore::open_with_limits`,
   `writer_loop`, `insert`, `delete`, `open_connection`, `migrate`, and the
   `#[cfg(test)]` helpers `temp_store`, `entry`, `enqueue`, `count_rows`,
   `assert_unique_keys`.
6. `git status --short`. Do not discard unrelated work.
7. Implement. Do not commit, push, or edit shell startup files unless asked.

## Goal

Prove:

1. An uncommitted writer transaction is rolled back on the next open; already
   committed rows survive; replaying `(session_id, event_sequence)` does not
   duplicate (`HIST-004` case 4; `HIST-012` Crash + Retry).
2. A corrupt `-wal`, `-shm`, or main db file does not make MBX unlink or
   rewrite the store into an empty success; open either recovers committed
   rows or returns a typed `HistoryError` (`HIST-004` case 5; `HIST-012`
   Storage failure).
3. Failure diagnostics stay command-text-free (`M-023`).

`ACK` / `record()` Ok still means queue accept, not commit. Do not change that.

## Out of scope (hard)

- Prompt-boundary write-ack optimization or weakening the 2 ms / 5 ms budget
- Foreign-user `seteuid` / second-account open
- Many-match prefix index, schema v2, `HIST-009` fuzzy ranking
- Real `kill -9` of the test process or of `mbx` (simulate crash; see Method)
- Repairing/dumping/rebuilding SQLite data (`VACUUM INTO`, `.recover`, deleting
  the db to "start clean" on corrupt open)
- Changing MBX1/MBX2 framing, field counts, or ACK meaning
- `set -euo pipefail` in sourced Bash modules
- `unsafe { env::set_var }`
- Reintroducing `MBX_DBG` or logging command text
- Marking `G2` or `HIST-007` complete
- Committing, pushing, or editing `~/.bashrc` unless the user asks

## Method

Add four tests at the **end** of `mod tests` in `crates/cli/src/storage.rs`
(before the closing `}` of that module). Reuse `temp_store`, `entry`,
`enqueue`, `count_rows`, `assert_unique_keys`. Do not add a new crate. Do not
split `storage.rs` in this slice.

**Crash analogue (K-1):** do **not** send `SIGKILL`. Open a raw
`rusqlite::Connection` on the same path, `BEGIN IMMEDIATE`, `INSERT` a new
row, then `drop(connection)` **without** `COMMIT`. That is the kill-9
mid-commit stand-in. Next `QueuedHistoryStore::open` must see only the
previously committed rows.

**Corrupt analogue (K-2–K-4):** after a committed store is dropped, overwrite
the target file with 4096 bytes of `0xFF` using `std::fs::write`. Then call
`QueuedHistoryStore::open`. Allowed outcomes:

- **Recover:** open `Ok`, committed count unchanged, keys unique, sentinel
  command still queryable; or
- **Fail closed:** open `Err` with `kind` in `{Open, Migrate, StorageFailure}`,
  the **main** `history.sqlite3` path still exists, byte length of that file is
  not zero, and MBX did not replace it with a fresh empty schema.

After a fail-closed open, the test may delete only the **corrupt sidecar**
(`-wal` or `-shm`) and reopen to prove committed rows are still in the main
file. It must **not** delete `history.sqlite3` to make the test pass.

WAL/SHM paths (copy from `QueuedHistoryStore::delete`):

```rust
fn sidecar(store: &Path, suffix: &str) -> PathBuf {
    store.with_file_name(format!(
        "{}{suffix}",
        store.file_name().map(|name| name.to_string_lossy()).unwrap_or_default()
    ))
}
// sidecar(path, "-wal") and sidecar(path, "-shm")
```

Committed INSERT for the crash connection must match `insert()`:

```sql
INSERT OR IGNORE INTO history (
    session_id, event_sequence, history_number, command_text, start_cwd,
    completed_at, status, duration_ms, host, user
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
```

Use sentinel command text `secret-wal-token` on the committed row. Every
`HistoryError` `Display` in these tests must not contain that string.
`history_failure_diagnostic` must stay `event=history_storage_error kind=...`
only.

If WAL was checkpointed away after `drop(store)`, K-2 must first recreate a
WAL: open raw connection, `PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;`,
insert+commit one **already-committed-key replay** or a new sequence, drop
without `PRAGMA wal_checkpoint`, then overwrite the `-wal` file. If the `-wal`
path still does not exist, `fs::write` it anyway (orphan WAL).

## Test cases (implement all)

| ID | Function name | Setup | Assert |
| --- | --- | --- | --- |
| K-1 | `crash_mid_transaction_rolls_back_and_retry_is_idempotent` | Commit `(s1,1)` via `QueuedHistoryStore` + drop. Raw connection: `BEGIN IMMEDIATE`, insert `(s1,2)` with command `echo uncommitted`, drop connection without `COMMIT`. Reopen store. Replay `(s1,1)`. Then enqueue `(s1,2)` for real and drop | After reopen: `count == 1`, command is `secret-wal-token`, not `echo uncommitted`. After replay of `(s1,1)`: still `count == 1`. After real `(s1,2)`: `count == 2`, `assert_unique_keys` |
| K-2 | `corrupt_wal_does_not_destroy_committed_store` | Commit `(s1,1)` `secret-wal-token`. Overwrite `-wal` with 4096 `0xFF` | Open recovers with `count == 1` **or** fail-closed (see Method). Main db file still exists. Error display has no sentinel. After removing only `-wal`/`-shm` if open failed, reopen `count == 1` |
| K-3 | `corrupt_shm_does_not_destroy_committed_store` | Same commit. Overwrite `-shm` with 4096 `0xFF` | Same allowed outcomes as K-2. Main db not unlinked |
| K-4 | `corrupt_main_db_fails_closed_without_replacing_the_file` | Commit `(s1,1)` `secret-wal-token`. Record `len` of main file. Overwrite **main** db with 4096 `0xFF` | `QueuedHistoryStore::open` is `Err`. Kind is `Open`, `Migrate`, or `StorageFailure`. `path` still exists. `metadata().len() != 0`. Display has no sentinel. Do **not** require recovery of the row; the file is the store and must not be replaced with a new empty v1 database |

## Product-code changes (only if a test fails)

Allowed:

- Map SQLite corrupt / not-a-database errors to `HistoryErrorKind::Open` or
  `StorageFailure` without putting SQL or command text in the diagnostic.
- Refuse to treat a non-empty unreadable file as "create a fresh store".
  `Connection::open` on a garbage file must not be followed by migrate that
  clobbers user data. If rusqlite creates an empty db on some garbage inputs,
  fail the open when `user_version` / `sqlite_master` cannot be read as v1
  **and** the file was already non-empty before open — preserve the bytes.
- Keep `INSERT OR IGNORE` as the uniqueness mechanism.

Not allowed:

- Deleting the db on open failure
- Auto-repair that rewrites rows
- Fire-and-forget ACK
- New protocol version
- Collapsing storage into the composition root

If you fix a confirmed defect, add or update `MISTAKES.md` in the same change
(search by cause; do not duplicate `M-031` / `M-032`).

## Documentation updates (same change)

Do **not** mark `G2` or `HIST-007` complete. Keep write-ack budget, many-match
prefix, and extra permission checks as remaining.

Update these to say crash/corrupt **correctness is recorded**, remaining G2 is
write-ack budget, permission beyond mode bits, many-match prefix:

- `docs/roadmap.md` — `HIST-007` evidence note, Not implemented list, History
  phase row, Immediate next work item 1, changelog row dated 2026-08-16
- `docs/architecture.md` history sidecar paragraph
- `docs/protocol-mbx2.md` status blurb
- `docs/adr/0005-history-storage.md` Validation plan
- `docs/benchmarks/2026-08-16-history-queries.md` and
  `docs/benchmarks/2026-08-16-history-write-ack.md` closing "does not complete
  G2" sentences
- This file: set Status to `complete` for K-1–K-4 once tests pass

Immediate next work after this slice: WAL/SHM `0600` plus never-more-permissive
(`docs/history-g2-permission-plan.md`). Not write-ack product optimization.

## Implementation checklist (do in this order)

1. Add `sidecar()` helper and K-1–K-4 in `crates/cli/src/storage.rs` `mod tests`.
2. Run `cargo test -p mbx --lib storage::`. Fix product code only per the
   policy above.
3. If a test proves open clobbers a corrupt file, fix that before adding more
   cases.
4. Reconcile the docs listed above. Do not claim the write-ack budget passed.
5. If you fixed a real defect, update `MISTAKES.md`.
6. Run `bash tests/run.bash` with unsandboxed `/dev/ptmx`
   (`required_permissions: ["all"]`).
7. Stop. Do not start permission, prefix-index, or write-ack work.

## Copy-paste skeleton (adapt names; keep asserts)

```rust
#[test]
fn crash_mid_transaction_rolls_back_and_retry_is_idempotent() {
    let (dir, path) = temp_store("k1");
    {
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        enqueue(
            &store,
            entry("s1", 1, "secret-wal-token", "/w", "2026-08-16T10:00:00Z"),
        );
    }
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE;")
            .unwrap();
        connection
            .execute(
                "INSERT OR IGNORE INTO history \
                 (session_id, event_sequence, history_number, command_text, start_cwd, \
                  completed_at, status, duration_ms, host, user) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    "s1",
                    2u64,
                    2i64,
                    "echo uncommitted",
                    "/w",
                    "2026-08-16T10:00:01Z",
                    0i32,
                    None::<u64>,
                    "host",
                    "user",
                ],
            )
            .unwrap();
        drop(connection);
    }
    {
        let store = QueuedHistoryStore::open(&path, 8).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        assert_eq!(store.recent(1).unwrap()[0].command_text, "secret-wal-token");
        enqueue(
            &store,
            entry("s1", 1, "secret-wal-token", "/w", "2026-08-16T10:00:00Z"),
        );
        enqueue(
            &store,
            entry("s1", 2, "echo committed-after-crash", "/w", "2026-08-16T10:00:02Z"),
        );
    }
    assert_eq!(count_rows(&path), 2);
    assert_unique_keys(&path);
    drop(dir);
}
```

For K-2/K-3, factor a helper:

```rust
fn assert_corrupt_sidecar_is_safe(path: &Path, sidecar_path: &Path) {
    std::fs::write(sidecar_path, vec![0xFF; 4096]).unwrap();
    match QueuedHistoryStore::open(path, 8) {
        Ok(store) => {
            assert_eq!(store.count().unwrap(), 1);
            assert_eq!(store.recent(1).unwrap()[0].command_text, "secret-wal-token");
            drop(store);
        }
        Err(error) => {
            let shown = error.to_string();
            assert!(
                matches!(
                    error.kind(),
                    HistoryErrorKind::Open
                        | HistoryErrorKind::Migrate
                        | HistoryErrorKind::StorageFailure
                ),
                "unexpected kind: {error}"
            );
            assert!(!shown.contains("secret-wal-token"), "{shown}");
            assert!(path.exists(), "corrupt sidecar must not unlink the db");
            let _ = std::fs::remove_file(sidecar_path);
            let store = QueuedHistoryStore::open(path, 8).unwrap();
            assert_eq!(store.count().unwrap(), 1);
        }
    }
    assert_unique_keys(path);
}
```

K-4 must **not** call `remove_file` on the main db and reopen expecting
success.

## Follow-on `G2` slices (not this change)

1. WAL/SHM `0600` plus never-more-permissive (`docs/history-g2-permission-plan.md`).
2. Foreign-user open (`HIST-004` case 7 remainder) when a second uid exists.
3. Many-match prefix latency (schema/index ADR).
4. Write-ack budget only after a test proves SQLite is on the prompt path.
