# HIST-007 slice: writer idle-flush for live reader visibility

Status: `complete` for V-1–V-3 (2026-08-16). Do not mark `G2` or `HIST-007` complete. After
this slice, remaining `G2` is still foreign-user open and the write-ack budget.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Writer idle-flush so live `count`/`search` see rows (this plan) | PTY `wait_for_count` stays at `count=0` while the helper is alive because `WRITER_BATCH_SIZE=32` holds an open transaction. Invariance evidence is claimed but `bash tests/run.bash` fails. No second uid. ACK stays queue-accept. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. This WSL user is uid 1000, not root; `sudo -n` needs a password; `newuidmap` is not installed; `unshare --map-user` still owns the file. Do not fake `seteuid` (process-wide, unsafe with the writer thread). Do not `apt install uidmap`. |
| 3 | Write-ack p95/p99 budget miss | W-1–W-4 already prove samples exist at prompt return before SQLite drain. Do not chase product-code latency unless a test proves the prompt waits on SQLite, samples contain command text, or ACK waits for commit. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-023` (no command-text diagnostics),
   `M-024` (history stays opt-in), `M-029`–`M-033` (do not undo under-cap prune
   skip, do not undo batching of a **busy** queue, lock retries, tighten-only
   chmod).
2. Read this file completely. Do not invent extra cases.
3. Read `docs/history-phase3a-contract.md` durability: queue acknowledgement is
   not commit; writer “batches where practical”; no prompt-path SQLite wait.
4. Read ADR 0005 section 7: prompt enqueues; writer commits batches of 32;
   `ACK` is not commit.
5. Read `writer_loop` in `crates/cli/src/storage.rs` and
   `wait_for_count` in `crates/pty/tests/common/mod.rs`.
6. `git status --short`. Do not discard unrelated work.
7. Implement. Do not commit, push, or edit shell startup files unless asked.

## Goal

1. While a writer is **alive**, an external reader (`mbx history count` or a
   second `QueuedHistoryStore`) can see rows after the queue has gone idle,
   without waiting for 32 inserts or helper shutdown.
2. A **busy** queue still batches to `WRITER_BATCH_SIZE` (`M-030`). Do not
   autocommit every insert.
3. `ACK` / `record()` Ok still means queue accept, not commit. The prompt path
   must not wait on `COMMIT`.
4. PTY `history_invariance` `wait_for_count` timeouts at `count=0` go away
   **because the writer flushes**, not because the harness was weakened.

## Out of scope (hard)

- `seteuid`, `setuid` in the test process, second Unix account, `chown`,
  installing `uidmap`, `sudo`
- Changing `WRITER_BATCH_SIZE` (keep `32`)
- Changing `wait_for_count` timeout, poll interval, or making it attach to the
  helper’s writer connection
- Making MBX2 ACK wait for SQLite commit
- Prompt-boundary write-ack optimization or weakening 2 ms / 5 ms
- Foreign-user open, fuzzy ranking, schema v3, FTS
- Splitting `storage.rs`
- `set -euo pipefail` in sourced Bash modules
- Reintroducing `MBX_DBG` or logging command text
- Marking `G2` or `HIST-007` complete
- Claiming foreign-user open or the write-ack budget passed
- Committing, pushing, or editing `~/.bashrc` unless the user asks

## Method

**Root cause (do not “fix” in the PTY harness):**
`writer_loop` only `COMMIT`s at `pending >= 32` or `Shutdown`. PTY tests admit
1–2 commands, ACK returns, then `wait_for_count` opens a **separate** read-only
`mbx history count`. The helper’s open transaction is invisible. Last count is
`0` until process teardown, which the test never reaches.

**Allowed product change:** when `pending > 0` and `receiver.try_recv()` returns
`Empty`, `COMMIT` the partial batch, set `pending = 0`, then **block** on
`recv()`. Do **not** prune on this idle flush (prune stays on full-batch commit
and Shutdown, `M-029`).

Busy ingest: the sync channel stays non-empty, so `try_recv` keeps returning
`Write` until 32, then the existing full-batch `COMMIT` + prune path runs.

Import `TryRecvError` next to the existing `mpsc` import. Do not add a crate.

Reuse `temp_store`, `entry`, `enqueue`. Sentinel for any new diagnostic
assertion: `secret-idle-token`. `history_failure_diagnostic` stays
`event=history_storage_error kind=...` only.

## Test cases (implement all)

Add V-1–V-2 at the end of `crates/cli/src/storage.rs` `mod tests`. V-3 is the
existing PTY suite, not a new test file.

| ID | Function name | Setup | Assert |
| --- | --- | --- | --- |
| V-1 | `idle_writer_commits_one_row_for_a_live_external_reader` | Open store A. `enqueue` one row with `secret-idle-token`. **Do not drop A.** Poll store B (`QueuedHistoryStore::open` + `count`/`recent`) for up to 2 s | `count == 1` while A is still alive. `recent(1)[0].command_text` is the sentinel. Then drop A |
| V-2 | `idle_writer_commits_a_partial_batch_under_writer_batch_size` | Open store A. `enqueue` 8 rows (well under 32). Do not drop A. Poll B for up to 2 s | `count == 8` while A is alive. `assert_unique_keys`. Drop A |
| V-3 | PTY `history_invariance` | Unsandboxed `/dev/ptmx` | `wait_for_count` reaches the expected count. Do not edit `wait_for_count` to hide a still-zero count |

Keep `writer_thread_persists_batches_until_shutdown` and the 100k ignored
corpus test as-is. Do not add a second 100k loader.

If V-1 still sees `count=0` after 2 s, the idle flush is wrong; fix
`writer_loop` before touching PTY helpers.

## Product-code changes (only as needed for V-1–V-3)

Allowed:

- `writer_loop` idle `COMMIT` when `try_recv` is `Empty` and `pending > 0`
- A small private helper to `COMMIT`/`ROLLBACK`/`pending=0` if that avoids
  duplicating the existing failure path (`M-031`: failed `COMMIT` still
  `ROLLBACK`s before clearing `pending`)
- `TryRecvError` import

Not allowed:

- `WRITER_BATCH_SIZE = 1` (or any other value)
- `recv_timeout` busy-loops, sleeps on the prompt path, or `ACK` after commit
- Prune on every idle flush
- Deleting the db on commit failure
- Changing MBX2 ACK meaning
- Collapsing storage into the composition root

If idle flush reintroduces unbounded 100k ingest (autocommit-every-row), that
is `M-030` recurring: stop, restore batching of a non-empty queue, and update
`M-030` evidence rather than adding a duplicate ID.

## Documentation updates (same change)

Do **not** mark `G2` or `HIST-007` complete.

Update these to say live-reader idle-flush **correctness is recorded**, remaining
G2 is still foreign-user open and write-ack budget:

- `docs/roadmap.md` — `HIST-007` evidence note, History phase row, Immediate
  next work item 1, changelog row dated 2026-08-16
- `docs/architecture.md` history sidecar paragraph (`ACK` still queue-accept;
  writer now idle-flushes partial batches)
- `docs/protocol-mbx2.md` status blurb (PTY count visibility unblocked; do not
  claim ACK is commit)
- `docs/adr/0005-history-storage.md` section 7: batches of 32 **or** idle
  flush when the queue is empty; prune still after full batches / shutdown
- This file: set Status to `complete` for V-1–V-3 once storage + PTY invariance
  pass

Immediate next work after this slice: foreign-user open when a second host uid
is actually available. Not write-ack product optimization. Not fuzzy ranking.

## Implementation checklist (do in this order)

1. Change `writer_loop` as specified. Keep `WRITER_BATCH_SIZE` at 32.
2. Add V-1 and V-2.
3. Run `cargo test -p mbx --lib storage::`.
4. Run `cargo test -p mbx-pty --test history_invariance` unsandboxed
   (`required_permissions: ["all"]`).
5. Reconcile the docs listed above.
6. If you fixed a real defect, update `MISTAKES.md` (search by cause; idle
   flush that autocommits a busy queue is `M-030`, not a new ID).
7. Run `bash tests/run.bash` with unsandboxed `/dev/ptmx`. Storage + corpus +
   PTY invariance must pass. Concurrent 255-vs-256 remains the known WAL busy
   flake if it appears under parallel load.
8. Stop. Do not start foreign-user or write-ack work.

## Copy-paste skeleton (adapt names; keep asserts)

```rust
fn wait_for_external_count(path: &Path, expected: u64) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Ok(reader) = QueuedHistoryStore::open(path, 8) {
            if reader.count().unwrap() == expected {
                return;
            }
        }
        if std::time::Instant::now() >= deadline {
            panic!("external reader never reached count {expected}");
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
        entry("s1", 1, "secret-idle-token", "/w", "2026-08-16T16:00:00Z"),
    );
    wait_for_external_count(&path, 1);
    let reader = QueuedHistoryStore::open(&path, 8).unwrap();
    assert_eq!(reader.recent(1).unwrap()[0].command_text, "secret-idle-token");
    drop(reader);
    drop(writer);
    drop(dir);
}
```

Writer idle path (structure only; keep existing Write/Shutdown bodies):

```rust
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
                // COMMIT partial batch; ROLLBACK on failure (M-031); pending = 0;
                // do not prune here
                match receiver.recv() {
                    Ok(message) => message,
                    Err(_) => break,
                }
            }
            Err(TryRecvError::Disconnected) => {
                // same COMMIT/ROLLBACK as Shutdown for pending > 0
                break;
            }
        }
    };
    match message { /* existing Write / Shutdown */ }
}
```

## Follow-on `G2` slices (not this change)

1. Foreign-user open (`HIST-004` case 7 remainder) when a second **host** uid
   can open the file (root/`nobody` child, or `newuidmap` subuids). Not
   `unshare --map-user` of the same outer uid.
2. Write-ack budget only after a test proves SQLite is on the prompt path.
