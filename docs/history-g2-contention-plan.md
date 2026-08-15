# HIST-007 slice: concurrent writer contention (cases 1–3, 6)

Status: `complete` for concurrent-writer contention cases 1–3 and 6 (2026-08-16).
Prompt-boundary write-ack PTY, WAL crash/corrupt, migration-on-100k, many-match
prefix latency, and foreign-user permission checks remain later `G2` slices.

## Goal

Prove the existing per-session writer + SQLite WAL contract under concurrent
use, without PTY and without a second Unix user:

1. Distinct `QueuedHistoryStore` instances can write the same file concurrently.
2. `(session_id, event_sequence)` remains unique; retries do not duplicate.
3. A reader can `recent` / `exact_prefix` / `by_cwd` / `count` while a writer
   is inserting and pruning.
4. A full bounded queue returns `QueueFull` without blocking, and later
   accepted records still drain.

This is `HIST-004` contention cases 1, 2, 3, and 6 at **correctness** scale
(hundreds of rows, not 100k). Do not treat in-process `record()` latency as
the `G2` prompt-boundary write budget.

## Out of scope

- Prompt-boundary write-ack PTY (p95 < 2 ms / p99 < 5 ms)
- Case 4 writer `kill -9` mid-commit, case 5 corrupt WAL/SHM, case 8 100k
  migration, case 7 foreign-user open
- Many-match `git` prefix index / schema v2
- Fuzzy ranking, repository context, editor UI, default-on capture
- Marking `G2` or `HIST-007` complete
- Changing MBX1/MBX2 framing
- `set -euo pipefail` in sourced Bash modules
- Committing, pushing, or editing the user's shell startup files unless asked

## Method

Add focused tests in `crates/cli/src/storage.rs` (or a sibling `#[cfg(test)]`
module under `crates/cli/src/` if `storage.rs` is already hard to scan). Drive
two or more `QueuedHistoryStore::open` / `open_with_limits` handles on one
temp SQLite path. Reuse `HistoryEntry` construction from existing storage
tests. Capture retention at `open_with_limits` (do not mutate process env
from parallel tests; `M-029` / retention races).

Busy-timeout is 100 ms (`BUSY_TIMEOUT_MS`). Tests must retry `record()` on
`QueueFull` with `thread::yield_now()`, then drop stores so `Shutdown` drains.
Never print command text. Never write `.bash_history`.

## Test cases

| ID | Case | Assert |
| --- | --- | --- |
| C-1 | 8 stores, distinct `session_id`, 32 records each, unique sequences | After all drops, `count == 256`; no duplicate `(session_id, event_sequence)` |
| C-2 | Two stores, different session IDs, overlapping wall-clock writes | Both sessions' rows present; `INSERT OR IGNORE` keeps uniqueness |
| C-3 | One store inserting 64 rows with `open_with_limits(..., max_rows=40, days=36500)` while another store loops `recent(10)` / `count()` | Reader never panics; after writer drop, `count <= 40`; table still exists |
| C-4 | Replay the same `(session_id, event_sequence)` from a second store after the first committed | `count` unchanged (idempotent retry) |
| C-6 | `open(..., queue_capacity=2)` then 8 rapid `record()` calls | At least one `QueueFull`; `record()` returns without sleeping on SQLite; after drop, committed prefix is `0..=2` plus any drained extras, never more than 8, never a hang |

## Edge cases

- Two stores on one file is the production topology (one writer connection per
  helper session). Do not invent a shared daemon.
- `ACK` / `record()` Ok means queue accept, not commit. Poll `count()` after
  `drop(store)` (join on Shutdown), not immediately after `record()`.
- `BEGIN IMMEDIATE` can fail under WAL contention (`M-031`). That is acceptable
  loss of enhancement data. Tests must not require every enqueued row to land
  if the writer traced a begin/commit failure; they **must** require uniqueness
  of whatever did land, a live store, and no deadlock.
- Prefer asserting `count == 256` for C-1. If WAL `BUSY` drops some rows, assert
  `count > 0`, uniqueness of remaining keys, and that both session ID prefixes
  appear — then record that as remaining `G2` evidence, do not weaken the
  durability contract in docs.
- Retention prune runs after a successful batch commit of 32. C-3 must use
  more than `WRITER_BATCH_SIZE` rows so prune actually runs.
- Do not use `unsafe { env::set_var }` for retention in parallel tests.
- Command text in concurrent rows must stay parameterized (reuse short `cmd N`
  strings; no need for a new hostile corpus).

## Implementation checklist

1. Persist this plan (already done). Update `docs/roadmap.md` immediate next
   work and the `HIST-007` evidence note in the same change. Do not mark `G2`
   complete.
2. Implement C-1, C-2, C-3, C-4, C-6 as storage tests.
3. Product-code changes only if a test finds a deadlock, duplicate key, or
   blocking `record()`. Keep writer/search/control ports; do not collapse
   storage into the composition root.
4. Run `cargo test -p mbx --lib storage::` then `bash tests/run.bash`.
   PTY tests need unsandboxed `/dev/ptmx` (`required_permissions: ["all"]`).
5. If a confirmed defect is fixed, add or update `MISTAKES.md` in the same
   change. Do not duplicate an existing cause.

## Follow-on `G2` slices

Prompt-boundary write-ack PTY; WAL crash/corrupt + restart uniqueness; v0→v1
migration; many-match prefix latency; permission checks beyond mode bits.
