# HIST-007 slice: store permissions beyond dir+db mode bits

Status: `complete` for WAL/SHM `0600` and never-more-permissive cases P-1–P-4
(2026-08-16). Do not mark `G2` or `HIST-007` complete. Foreign-user open,
many-match prefix latency, and write-ack budget remain.

## Why this slice (do not pick a different leftover)

Remaining `G2` items, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | WAL/SHM `0600` + never-more-permissive (this plan) | ADR 0005 §5 already requires it. `store_files_are_user_only` only checks dir + main db. Product `set_permissions` always applies `0700`/`0600`, which can **widen** `0400`/`0000`. No second uid. Storage tests only. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a second Unix uid, `seteuid`, or user namespace. Do not fake it. Leave remaining. |
| 3 | Many-match `git` prefix (~61 ms p95) | Schema/index or query-plan change. Likely ADR. Not a small test-only slice. |
| 4 | Write-ack p95/p99 budget miss | Correctness is recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite, samples contain command text, or ACK waits for commit. |
| — | PTY `wait_for_count` staying 0 under `WRITER_BATCH_SIZE=32` | Known helper-batch visibility issue; not this slice. Do not change batch size, ACK meaning, or `wait_for_count`. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-023` (no command-text diagnostics),
   `M-024` (history stays opt-in), `M-029`–`M-032` (do not undo writer batching
   or lock retries).
2. Read this file completely. Do not invent extra cases.
3. Read `docs/adr/0005-history-storage.md` section 5 (Storage, permissions, and
   lifecycle), especially: directory `0700`; database, WAL, and SHM `0600`;
   **existing files are never made more permissive**.
4. Read `docs/benchmarks/history-budgets.md` contention case 7.
5. Read `crates/cli/src/storage.rs`: `DIR_MODE`, `FILE_MODE`, `create_store_dir`,
   `open_connection`, `QueuedHistoryStore::delete` sidecar naming, and
   `store_files_are_user_only`. Reuse the existing test helper `sidecar()`.
6. `git status --short`. Do not discard unrelated work.
7. Implement. Do not commit, push, or edit shell startup files unless asked.

## Goal

Prove:

1. After a writer open that creates WAL, the directory is `0700` and the
   database, `-wal`, and `-shm` files are `0600` when they exist (ADR 0005 §5;
   `HIST-004` case 7 mode bits).
2. A world-readable/world-writable existing store is **tightened** (bits only
   removed) to those maxima; it is not left group/other-accessible.
3. A more restrictive existing mode (`0400` or `0000`) is **not widened** to
   `0600`/`0700`. Open may fail closed; the file is not replaced with a fresh
   `0600` v1 database (`M-023`: errors stay command-text-free).

`ACK` / `record()` Ok still means queue accept, not commit. Do not change that.

## Out of scope (hard)

- `seteuid`, `setuid`, a second Unix account, user namespaces, ACLs, `chown`
- Many-match prefix index, schema v2, `HIST-009` fuzzy ranking
- Prompt-boundary write-ack optimization or weakening the 2 ms / 5 ms budget
- Changing `WRITER_BATCH_SIZE`, `wait_for_count`, or MBX2 ACK meaning
- Changing MBX1/MBX2 framing or field counts
- `set -euo pipefail` in sourced Bash modules
- `unsafe { env::set_var }` except through the existing `history_env_lock`
  helper already in this file
- Reintroducing `MBX_DBG` or logging command text
- Marking `G2` or `HIST-007` complete
- Claiming foreign-user open is done
- Committing, pushing, or editing `~/.bashrc` unless the user asks
- Splitting `storage.rs` (already over 1k lines; this slice adds a small helper
  plus four tests)

## Method

Add four tests at the **end** of `mod tests` in `crates/cli/src/storage.rs`
(before the closing `}` of that module). Reuse `temp_store`, `entry`, `enqueue`,
`sidecar`, `count_rows`. Do not add a new crate.

Keep `store_files_are_user_only` as-is (dir + main db after drop). The new tests
cover WAL/SHM and the never-more-permissive rule.

**Tighten, never widen.** Product code must not `set_permissions(0700/0600)`
unconditionally. Use:

```text
new_mode = (current & 0o777) & max_mode
```

Only `chmod` when `new_mode != current`. That removes group/other bits and
never adds owner/group/other bits.

Call the helper from `open_connection` **after** `ensure_wal_mode` and
`migrate` so `-wal`/`-shm` exist. Also tighten the parent directory (replace
the unconditional `set_permissions(DIR_MODE)` in `create_store_dir`).

Reuse sidecar naming from `QueuedHistoryStore::delete` / test `sidecar()`:

```rust
// sidecar(path, "-wal") and sidecar(path, "-shm")
```

Use sentinel command text `secret-perm-token` on any stored row. Every
`HistoryError` `Display` in these tests must not contain that string.
`history_failure_diagnostic` must stay `event=history_storage_error kind=...`
only.

On Linux, the owner can `chmod` a `0000` file and can `unlink` it via a writable
parent. TempDir cleanup therefore still works. Do not `chmod` the temp parent
to `0000`.

## Test cases (implement all)

| ID | Function name | Setup | Assert |
| --- | --- | --- | --- |
| P-1 | `wal_and_shm_files_are_user_only` | `QueuedHistoryStore::open`, `enqueue` one `secret-perm-token` row, **keep the store alive** | Parent dir `0700`. Main db `0600`. `-wal` exists and is `0600`. `-shm` exists and is `0600`. Then drop |
| P-2 | `world_accessible_store_is_tightened` | Commit one sentinel row and drop. `chmod` parent `0o777`, main db `0o644`, `-wal`/`-shm` `0o666` if they exist. Reopen | Open `Ok`. Dir `0700`. Db `0600`. Existing `-wal`/`-shm` `0600`. Count is 1. Sentinel still queryable. Error path N/A |
| P-3 | `restrictive_file_is_not_made_more_permissive` | Commit sentinel, drop. `chmod` main db `0o400`. Reopen | Mode is still `0400` (not `0600`). Open may `Err` (`Open` / `Migrate` / `StorageFailure`) or `Ok`; if `Ok`, must still be `0400` and count 1. File exists, len ≠ 0, not replaced by a new empty v1 db. Display/diagnostic have no sentinel |
| P-4 | `unreadable_store_fails_closed_without_widening` | Commit sentinel, drop. Record len. `chmod` main db `0o000`. Reopen | Open is `Err` with kind `Open`, `Migrate`, or `StorageFailure`. Mode is still `0000`. Path exists. `metadata().len()` equals the recorded len (not a fresh db). Display/diagnostic have no sentinel. Do **not** `chmod` back to `0600` to make open succeed |

If P-1 cannot see `-wal`/`-shm` while the writer connection is live, that is a
product bug in WAL setup, not a reason to skip the assert. Do not checkpoint
away the sidecars before the mode check.

## Product-code changes (only if a test fails)

Allowed:

- Add `tighten_mode(path, max_mode)` (skip missing paths) and
  `restrict_store_permissions(store_path)` next to `create_store_dir`.
- Replace unconditional `set_permissions(DIR_MODE)` / `FILE_MODE` with tighten.
- After `ensure_wal_mode` + `migrate`, tighten dir, db, `-wal`, and `-shm`.
- Map permission `io::Error` to `HistoryErrorKind::Open` without SQL or command
  text in the diagnostic.

Not allowed:

- `chmod` to a mode with bits not already present (`new_mode = max_mode`
  unconditionally)
- Deleting the db on open failure
- `chown` / `seteuid` / ACLs
- Fire-and-forget ACK
- New protocol version
- Collapsing storage into the composition root
- Changing `WRITER_BATCH_SIZE`

If you fix a confirmed defect (unconditional chmod that widens, or WAL/SHM left
at umask `0644`), add or update `MISTAKES.md` in the same change. Search by
cause; do not duplicate `M-032`. Suggested shape if widening is confirmed:
unconditional `set_permissions(0600)` treated “user-only” as “always 0600”
rather than “never more permissive.”

## Documentation updates (same change)

Do **not** mark `G2` or `HIST-007` complete. Do not claim foreign-user open.

Update these to say WAL/SHM `0600` and never-more-permissive **correctness is
recorded**, remaining G2 is foreign-user open, many-match prefix, write-ack
budget:

- `docs/roadmap.md` — `HIST-007` evidence note, Not implemented list, History
  phase row, Immediate next work item 1, changelog row dated 2026-08-16
- `docs/architecture.md` history sidecar paragraph
- `docs/protocol-mbx2.md` status blurb
- `docs/adr/0005-history-storage.md` Validation plan
- `docs/benchmarks/2026-08-16-history-queries.md` and
  `docs/benchmarks/2026-08-16-history-write-ack.md` closing “does not complete
  G2” sentences
- This file: set Status to `complete` for P-1–P-4 once tests pass

Immediate next work after this slice: many-match prefix (separate plan) **or**
foreign-user open if a second uid is available. Not write-ack product
optimization.

## Implementation checklist (do in this order)

1. Add `tighten_mode` / `restrict_store_permissions` and call them from
   `create_store_dir` and `open_connection` (after WAL exists).
2. Add P-1–P-4 at the end of `mod tests`. Reuse `sidecar()`.
3. Run `cargo test -p mbx --lib storage::`. If P-1 fails because WAL is `0644`,
   fix product chmod, not the test. If P-3/P-4 fail because open widens then
   succeeds, fix tighten-only before adding more cases.
4. Reconcile the docs listed above. Do not claim the write-ack budget passed.
5. If you fixed a real defect, update `MISTAKES.md`.
6. Run `bash tests/run.bash` with unsandboxed `/dev/ptmx`
   (`required_permissions: ["all"]`). PTY `wait_for_count` timeouts at `count=0`
   with store files present are the known batch-visibility issue; do not “fix”
   them in this slice. Storage tests must pass.
7. Stop. Do not start prefix-index, foreign-user, or write-ack work.

## Copy-paste skeleton (adapt names; keep asserts)

```rust
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
            HistoryErrorKind::Open | HistoryErrorKind::Migrate | HistoryErrorKind::StorageFailure
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
    assert!(wal.exists(), "WAL sidecar must exist while the writer is live");
    assert!(shm.exists(), "SHM sidecar must exist while the writer is live");
    assert_eq!(mode_of(&wal), 0o600);
    assert_eq!(mode_of(&shm), 0o600);
    drop(store);
    drop(dir);
}

#[test]
fn world_accessible_store_is_tightened() {
    let (dir, path) = temp_store("p2");
    commit_perm_sentinel(&path);
    chmod(dir.path(), 0o777);
    chmod(&path, 0o644);
    for suffix in ["-wal", "-shm"] {
        let sidecar_path = sidecar(&path, suffix);
        if sidecar_path.exists() {
            chmod(&sidecar_path, 0o666);
        }
    }
    let store = QueuedHistoryStore::open(&path, 8).unwrap();
    assert_eq!(mode_of(dir.path()), 0o700);
    assert_eq!(mode_of(&path), 0o600);
    for suffix in ["-wal", "-shm"] {
        let sidecar_path = sidecar(&path, suffix);
        if sidecar_path.exists() {
            assert_eq!(mode_of(&sidecar_path), 0o600);
        }
    }
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
```

`PermissionsExt` is already imported in `mod tests`.

## Follow-on `G2` slices (not this change)

1. Foreign-user open (`HIST-004` case 7 remainder) when a second uid exists.
2. Many-match prefix latency (schema/index ADR).
3. Write-ack budget only after a test proves SQLite is on the prompt path.
