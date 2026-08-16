# HIST-007 slice: 100k-row v1→v2 migration (HIST-004 case 8)

Status: `complete` for M-1–M-3 (2026-08-16). Do not mark `G2` or `HIST-007`
complete. After this slice, remaining `G2` is still foreign-user open and the
write-ack budget.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | 100k-row v1→v2 migrate (`HIST-004` case 8) (this plan) | Q-A migrated one v1 row. Case 8 still requires empty→v1→v2 on the 100k corpus. No second uid. No PTY. No ACK change. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. This WSL user is uid 1000; `sudo -n` needs a password; `newuidmap` is missing; `unshare --map-user` still owns the file. Do not fake `seteuid`. Do not `apt install uidmap`. |
| 3 | Write-ack p95/p99 budget miss | W-1–W-4 prove samples exist at prompt return before SQLite drain. Do not chase product-code latency unless a test proves the prompt waits on SQLite, samples contain command text, or ACK waits for commit. |
| — | `FND-001` / `G0` CI URL | Needs a linked GitHub Actions run, not a storage change. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-023`–`M-034` (no command-text
   diagnostics, history stays opt-in, keep busy-queue batching, lock retries,
   tighten-only chmod, idle-flush).
2. Read this file completely. Do not invent extra cases.
3. Read `docs/benchmarks/history-budgets.md` contention case 8.
4. Read ADR 0008 (forward-only v1→v2, covering index, no FTS).
5. Read `try_migrate`, `SCHEMA_V1`, `SCHEMA_V2_INDEX`, and
   `schema_v1_store_migrates_to_v2_prefix_index` in
   `crates/cli/src/storage.rs`. Read `entry_at` / `CORPUS_SIZE` /
   `load_100k_and_measure_query_percentiles` in `crates/cli/src/corpus.rs`.
6. `git status --short`. Do not discard unrelated work.
7. Implement. Do not commit, push, or edit shell startup files unless asked.

## Goal

1. A v1-only store filled with the seeded 100k corpus migrates to schema v2 on
   `QueuedHistoryStore::open` without dropping rows (`HIST-004` case 8,
   `HIST-006`).
2. After migrate: `user_version == 2`, `history_prefix` and
   `history_prefix_completed` both exist, `count == 100000`, newest-first
   prefix still works.
3. Empty stores still open at v2 (Q-B). One-row v1 migrate (Q-A) still passes.
4. Record migrate wall time in a dated benchmark file. Do not weaken budgets.
   Do not add schema v3.

`ACK` / `record()` Ok still means queue accept, not commit.

## Out of scope (hard)

- `seteuid`, `sudo`, `uidmap`, second Unix account, `chown`
- Prompt-boundary write-ack optimization or weakening 2 ms / 5 ms
- Changing `WRITER_BATCH_SIZE`, `wait_for_count`, or MBX2 ACK meaning
- FTS, extra columns, dropping `history_prefix`, `VACUUM`
- Splitting `storage.rs`
- Duplicating the 100k query-percentile loader as an always-on unit test
- Marking `G2` or `HIST-007` complete
- Claiming foreign-user open or the write-ack budget passed
- Committing, pushing, or editing `~/.bashrc` unless the user asks

## Method

Do **not** fill 100k through `QueuedHistoryStore` while `SCHEMA_VERSION` is 2;
that lands on v2 before the migrate under test.

Fill a **raw v1** file:

1. `rusqlite::Connection::open`, `PRAGMA journal_mode=WAL`
2. `SCHEMA_V1` only (no `SCHEMA_V2_INDEX`)
3. `PRAGMA user_version = 1`
4. `INSERT OR IGNORE` corpus rows (reuse `crate::corpus::entry_at` /
   `CORPUS_SEED` / `CORPUS_SIZE`)
5. Close the connection
6. `QueuedHistoryStore::open_with_limits` (retention high enough not to prune
   100k: `1_000_000` rows / `36_500` days, same as the ignored query bench)
7. Drop the store (idle-flush + shutdown commit)

Reuse Q-A insert SQL. Batch inserts in a single `BEGIN IMMEDIATE` /
`COMMIT` when filling v1 so debug setup is not unbounded (`M-030` lesson).

Keep Q-A (one row) as the always-on regression. Case 8 is **release + ignored**,
like `load_100k_and_measure_query_percentiles`. Do not make `cargo test`
always load 100k.

Optional always-on extra (only if it stays fast): a 64-row v1 fill that is
not Q-A. Prefer not to add it if Q-A plus the ignored 100k already cover the
contract.

## Test cases (implement all)

| ID | Function name | Setup | Assert |
| --- | --- | --- | --- |
| M-1 | Keep `schema_v1_store_migrates_to_v2_prefix_index` | Unchanged Q-A | Still passes |
| M-2 | `schema_v1_100k_corpus_migrates_to_v2` in `crates/cli/src/corpus.rs` `mod tests`, `#[ignore = "HIST-004 case 8 100k v1→v2 migrate; run via scripts/benchmark-history-migrate.bash"]` | Raw v1 WAL store, insert `CORPUS_SIZE` `entry_at` rows, then `QueuedHistoryStore::open_with_limits` | `user_version == 2`. Both prefix indexes exist. `count == CORPUS_SIZE`. `exact_prefix("git", 50)` nonempty and newest-first (`completed_at` descending). File path exists, `len != 0`. Print `area=history_migrate_v1_v2 rows=100000 elapsed_ms=...` |
| M-3 | `scripts/benchmark-history-migrate.bash` | `cargo test -p mbx --release --lib corpus::tests::schema_v1_100k_corpus_migrates_to_v2 -- --ignored --exact --nocapture` | Writes/prints the elapsed line. Do not fail the script on write-ack budgets |

Record env + elapsed in `docs/benchmarks/2026-08-16-history-migrate.md` (or the
next UTC date if that name exists).

Sentinel: do not log command text. Corpus rows already include hostile SQL;
inertness stays covered by existing CORP tests.

## Product-code changes (only as needed for M-1–M-3)

Allowed:

- Test + benchmark script + dated results file
- Tiny `pub(crate)` test helper to run `SCHEMA_V1` **only if** corpus tests
  cannot see `SCHEMA_V1` (it is private in `storage.rs`). Prefer inserting from
  `storage::tests` if that is simpler; putting M-2 in `corpus.rs` is allowed
  if you `pub(crate)` a `create_v1_store(path)` helper. Do not move migrate
  policy into corpus.

Not allowed:

- Schema v3
- Dropping rows or `history_prefix`
- Changing ACK meaning or `WRITER_BATCH_SIZE`
- Filling 100k via `QueuedHistoryStore` then claiming that tested v1→v2
- Deleting the db on migrate failure

If migrate drops corpus rows, fix `try_migrate` before adding more cases and
update `MISTAKES.md` (search by cause; do not duplicate `M-032`).

## Documentation updates (same change)

Do **not** mark `G2` or `HIST-007` complete.

Update these to say case-8 100k v1→v2 migrate **is recorded**, remaining G2 is
still foreign-user open and write-ack budget:

- `docs/roadmap.md` — `HIST-007` evidence note, Immediate next work item 1,
  changelog row dated 2026-08-16
- `docs/adr/0005-history-storage.md` Validation plan
- `docs/adr/0008-history-prefix-index.md` Validation
- `docs/benchmarks/history-budgets.md` case 8 note
- `docs/history-g2-contention-plan.md` deferred case-8 sentence
- This file: Status `complete` for M-1–M-3 once the ignored test + dated file
  land

Immediate next work after this slice: foreign-user open when a second host uid
exists. Not write-ack product optimization.

## Implementation checklist (do in this order)

1. Add ignored M-2 + `scripts/benchmark-history-migrate.bash`.
2. Run Q-A: `cargo test -p mbx --lib schema_v1_store_migrates_to_v2`.
3. `cargo build --release --workspace` then the migrate script.
4. Write the dated benchmark file. Reconcile the docs listed above.
5. If you fixed a real migrate defect, update `MISTAKES.md`.
6. `bash tests/run.bash` unsandboxed. The ignored 100k test must not run inside
   the canonical suite. Concurrent 255-vs-256 remains the known WAL flake.
7. Stop. Do not start foreign-user or write-ack work.

## Copy-paste skeleton (adapt names; keep asserts)

```rust
#[test]
#[ignore = "HIST-004 case 8 100k v1→v2 migrate; run via scripts/benchmark-history-migrate.bash"]
fn schema_v1_100k_corpus_migrates_to_v2() {
    let started = Instant::now();
    let (dir, path) = temp_store("v1-100k");
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        connection.execute_batch(SCHEMA_V1).unwrap();
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
    }
    let store = QueuedHistoryStore::open_with_limits(&path, 8_192, 1_000_000, 36_500).unwrap();
    drop(store);
    let store = QueuedHistoryStore::open_with_limits(&path, 8_192, 1_000_000, 36_500).unwrap();
    assert_eq!(store.count().unwrap(), CORPUS_SIZE as u64);
    let git = store.exact_prefix("git", 50).unwrap();
    assert!(!git.is_empty());
    assert!(git.windows(2).all(|pair| pair[0].completed_at >= pair[1].completed_at));
    println!(
        "area=history_migrate_v1_v2 rows={CORPUS_SIZE} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    drop(store);
    drop(dir);
}
```

`SCHEMA_V1` is private. Either put this test in `storage.rs` and import corpus
helpers, or add `pub(crate) fn apply_schema_v1(connection: &Connection)`.

## Follow-on `G2` slices (not this change)

1. Foreign-user open when a second **host** uid can open the file.
2. Write-ack budget only after a test proves SQLite is on the prompt path.
