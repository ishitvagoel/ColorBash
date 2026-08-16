# HIST-007 slice: many-match exact-prefix latency (schema v2)

Status: `complete` for Q-A–Q-D. Do not mark `G2` or `HIST-007` complete. After
this slice, remaining `G2` is foreign-user open and the write-ack budget.

## Why this slice (do not pick a different leftover)

Remaining `G2` items, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Many-match `git` prefix (~61 ms p95) (this plan) | Recorded miss. Query sorts every `git*` match. Needs covering index + schema v2 + ADR. No second uid. No PTY. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a second Unix uid / user namespace. Do not fake `seteuid`. |
| 3 | Write-ack p95/p99 budget miss | Correctness is recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite, samples contain command text, or ACK waits for commit. |
| — | PTY `wait_for_count` staying 0 under `WRITER_BATCH_SIZE=32` | Known helper-batch visibility issue. Do not change batch size, ACK meaning, or `wait_for_count`. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-023` (no command-text diagnostics),
   `M-024` (history stays opt-in), `M-029`–`M-033` (do not undo writer batching,
   lock retries, or tighten-only chmod).
2. Read this file completely. Do not invent extra cases.
3. Read `docs/adr/0005-history-storage.md` sections 6 (schema/indexes) and 7
   (concurrency). Schema change needs **ADR 0008**, not a silent edit of 0005.
4. Read `docs/history-phase3a-contract.md` Query contract (exact-prefix
   `LIKE 'prefix%'` with NOCASE, bounded, parameterized).
5. Read `docs/benchmarks/2026-08-16-history-queries.md` (`history_query_prefix`
   vs `history_query_prefix_common`).
6. Read `crates/cli/src/storage.rs`: `SCHEMA_V1`, `exact_prefix`, `try_migrate`,
   `schema_creates_tables_and_migrations_run_once`, and
   `like_metacharacters_in_prefix_are_literal`. Read `SCHEMA_VERSION` in
   `crates/cli/src/history.rs`. Read the ignored
   `load_100k_and_measure_query_percentiles` in `crates/cli/src/corpus.rs`.
7. `git status --short`. Do not discard unrelated work.
8. Implement. Do not commit, push, or edit shell startup files unless asked.

## Goal

1. A covering prefix index so `exact_prefix("git", 50)` does not sort every
   `git*` row (`HIST-004` exact-prefix p95 < 10 ms on the 100k corpus).
2. Forward-only schema v1 → v2: existing stores migrate; empty stores land on
   v2; `user_version` is 2 after open (`HIST-006`).
3. Query results stay newest-first, bounded, parameterized, and
   LIKE-literal (`HIST-008`). Hostile prefix strings still cannot match-all.
4. Record a new dated benchmark file. If p95 is still ≥ 10 ms after the
   covering index, **record the miss**. Do not weaken the documented budget
   and do not add a second schema version in the same change.

`ACK` / `record()` Ok still means queue accept, not commit.

## Out of scope (hard)

- `seteuid`, second Unix account, ACLs, `chown`
- Prompt-boundary write-ack optimization or weakening 2 ms / 5 ms
- Changing `WRITER_BATCH_SIZE`, `wait_for_count`, or MBX2 ACK meaning
- Fuzzy ranking (`HIST-009`), repository context, default-on capture
- Changing MBX1/MBX2 framing or field counts
- `VACUUM`, rewriting command text, extra tables, FTS, trigrams
- Splitting `storage.rs`
- `set -euo pipefail` in sourced Bash modules
- Reintroducing `MBX_DBG` or logging command text
- Marking `G2` or `HIST-007` complete
- Claiming foreign-user open or the write-ack budget passed
- Committing, pushing, or editing `~/.bashrc` unless the user asks

## Method

**Root cause (do not “fix” with a smaller LIMIT):**
`history_prefix` is `(command_text COLLATE NOCASE)` only.
`ORDER BY completed_at DESC, event_sequence DESC LIMIT 50` still materializes
every `LIKE 'git%'` match. The corpus `git` prefix is a many-match case
(~61 ms p95). Selective prefixes already pass.

**Allowed schema v2 (copy this SQL):**

```sql
CREATE INDEX IF NOT EXISTS history_prefix_completed
    ON history (command_text COLLATE NOCASE, completed_at DESC, event_sequence DESC);
```

Keep `history_prefix` (v1). Do not drop it in this slice. Empty stores may
still run `SCHEMA_V1` then the v2 statement, or fold the new index into a
`SCHEMA_V2` batch that also creates v1 objects; either way `user_version`
must become 2.

**Migration:** bump `SCHEMA_VERSION` to `2` in `crates/cli/src/history.rs`.
Change `try_migrate` so:

- version `>= 2` → no-op
- version `0` / empty → create v1 objects + v2 index, set `user_version = 2`
- version `1` → `CREATE INDEX IF NOT EXISTS history_prefix_completed ...`,
  then `PRAGMA user_version = 2`
- keep `BEGIN IMMEDIATE`, version re-check, `ROLLBACK` on failure (`M-032`)

Do **not** rewrite `exact_prefix` SQL unless a focused `EXPLAIN QUERY PLAN`
test on a many-match store proves SQLite still cannot use the covering index.
If you must change SQL, keep `LIKE ?1` parameterized, `ESCAPE '\\'` when the
prefix contains `%`/`_`, and `LIMIT ?2`. Never concatenate prefix into SQL.

**ADR 0008** (`docs/adr/0008-history-prefix-index.md`): Status `Accepted` in
this same change (schema change requires an ADR). Context = many-match miss.
Decision = covering index, forward-only v1→v2, no FTS. Consequences =
existing v1 stores migrate on next writer open; query contract unchanged.

Reuse `temp_store`, `entry`, `enqueue`. Sentinel command text for any new
diagnostic assertion: `secret-prefix-token`. `history_failure_diagnostic`
stays `event=history_storage_error kind=...` only.

## Test cases (implement all)

Add Q-A–Q-C in `crates/cli/src/storage.rs` `mod tests`. Keep Q-D as the
existing ignored corpus test (do not duplicate the 100k loader).

| ID | Function name | Setup | Assert |
| --- | --- | --- | --- |
| Q-A | `schema_v1_store_migrates_to_v2_prefix_index` | Create a v1-only store: raw `rusqlite` `PRAGMA journal_mode=WAL`, run current `SCHEMA_V1`, `PRAGMA user_version = 1`, insert `(s1,1)` `git status`. Then `QueuedHistoryStore::open` | `user_version == 2`. `sqlite_master` contains `history_prefix_completed`. Count is 1. Command is still `git status`. File was not replaced (path exists, len ≠ 0) |
| Q-B | `empty_store_opens_at_schema_v2` | `QueuedHistoryStore::open` on a new path | `user_version == 2`. Both `history_prefix` and `history_prefix_completed` exist. `schema_creates_tables_and_migrations_run_once` still passes after you update it to expect version 2 |
| Q-C | `many_match_prefix_uses_covering_index_and_stays_newest_first` | Insert 64 rows: 48 `git …` with increasing `completed_at`, 16 `echo …`. `exact_prefix("git", 5)` | Result len is 5. First command is the newest `git` row. `echo` rows absent. `EXPLAIN QUERY PLAN` for the production `exact_prefix` SQL (no ESCAPE branch) mentions `history_prefix_completed`. LIKE `%`/`_` tests still pass |
| Q-D | Re-run `scripts/benchmark-history.bash` | Release 100k corpus | New file `docs/benchmarks/2026-08-16-history-prefix.md` (or next UTC date if that name exists). Record env + p50/p95/p99 for `history_query_prefix_common`. Compare to 10 ms. If still over, say so; do not edit `docs/benchmarks/history-budgets.md` to raise the budget |

Update `schema_creates_tables_and_migrations_run_once` to expect
`SCHEMA_VERSION == 2` and the new index name. Do not delete its other asserts.

Keep `like_metacharacters_in_prefix_are_literal` and corpus CORP-4/CORP-5
passing with no SQL change unless Q-C requires the EXPLAIN tweak above.

## Product-code changes (only as needed for Q-A–Q-C / Q-D)

Allowed:

- `SCHEMA_VERSION = 2`
- `try_migrate` step for v1→v2 creating `history_prefix_completed`
- Optional `exact_prefix` SQL tweak **only** if EXPLAIN shows the old plan
- ADR 0008

Not allowed:

- Dropping `history` rows or `history_prefix`
- FTS / extra columns / rewriting `command_text`
- Deleting the db on migrate failure
- Changing ACK meaning
- Collapsing storage into the composition root
- Changing `WRITER_BATCH_SIZE`

If migrate clobbers a v1 store, fix that before adding more cases and add
`MISTAKES.md` (search by cause; do not duplicate `M-032` / `M-033`).

## Documentation updates (same change)

Do **not** mark `G2` or `HIST-007` complete.

Update these to say many-match prefix **correctness + new percentiles are
recorded**, remaining G2 is foreign-user open and write-ack budget:

- `docs/roadmap.md` — `HIST-007` evidence note, Not implemented list, History
  phase row, Immediate next work item 1, changelog row dated 2026-08-16
- `docs/architecture.md` history sidecar paragraph
- `docs/protocol-mbx2.md` status blurb
- `docs/adr/0005-history-storage.md` section 6 indexes + Validation plan
  (point at ADR 0008; do not pretend 0005 always had the covering index)
- `docs/benchmarks/2026-08-16-history-queries.md` closing sentence: many-match
  follow-up is in the new dated file
- This file: set Status to `complete` for Q-A–Q-D once tests + bench land

Immediate next work after this slice: foreign-user open **or** write-ack only
if a test proves SQLite is on the prompt path. Not fuzzy ranking.

## Implementation checklist (do in this order)

1. Add ADR 0008 (Accepted) with covering-index decision.
2. Bump `SCHEMA_VERSION`, extend `try_migrate` / `SCHEMA_V1` path, add Q-A–Q-C.
3. Run `cargo test -p mbx --lib storage:: corpus::`.
4. Run `cargo build --release --workspace` then
   `MBX_BENCH_ITERATIONS=200 bash scripts/benchmark-history.bash`.
5. Write the dated benchmark file. Reconcile the docs listed above.
6. If you fixed a real defect, update `MISTAKES.md`.
7. Run `bash tests/run.bash` with unsandboxed `/dev/ptmx`
   (`required_permissions: ["all"]`). PTY `wait_for_count` timeouts at
   `count=0` are the known batch-visibility issue; do not “fix” them here.
   Storage + corpus tests must pass.
8. Stop. Do not start foreign-user or write-ack work.

## Copy-paste skeleton (adapt names; keep asserts)

```rust
#[test]
fn schema_v1_store_migrates_to_v2_prefix_index() {
    let (dir, path) = temp_store("qa");
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .unwrap();
        connection.execute_batch(SCHEMA_V1).unwrap();
        connection
            .execute("PRAGMA user_version = 1", [])
            .unwrap();
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
    assert_eq!(version, 2);
    let covering: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='index' AND name='history_prefix_completed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(covering, 1);
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
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN \
             SELECT session_id, event_sequence, history_number, command_text, start_cwd, \
             completed_at, status, duration_ms, host, user \
             FROM history WHERE command_text LIKE 'git%' \
             ORDER BY completed_at DESC, event_sequence DESC LIMIT 5",
            [],
            |row| row.get(3),
        )
        .unwrap();
    assert!(
        plan.to_ascii_lowercase()
            .contains("history_prefix_completed"),
        "expected covering index in plan: {plan}"
    );
    drop(store);
    drop(dir);
}
```

`EXPLAIN QUERY PLAN` column 3 is the detail string on this rusqlite/SQLite.
If the column index differs, print the row once in a failing assert and use
the detail field; do not skip the plan check.

For Q-B, assert version 2 on a fresh `temp_store` after `open` + `drop`.

## Follow-on `G2` slices (not this change)

1. Foreign-user open (`HIST-004` case 7 remainder) when a second uid exists.
2. Write-ack budget only after a test proves SQLite is on the prompt path.
