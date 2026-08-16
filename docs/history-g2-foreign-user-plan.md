# HIST-007 slice: foreign-user open (`HIST-004` case 7)

Status: `complete` for F-1–F-4 (2026-08-16). Do not mark `G2` or `HIST-007`
complete. Write-ack budget remains.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Foreign-user open (this plan) | `HIST-004` case 7 remainder. Cloud host and GitHub Actions `ubuntu-latest` have passwordless `sudo` and a real foreign uid (`nobody` / 65534). Probe: `sudo -n -u nobody` cannot read a `0600` file owned by the store creator. Not `seteuid`, not `unshare --map-user`. |
| 2 | Write-ack p95/p99 budget miss | W-1–W-4 correctness recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite. |
| — | PRM-002 wrap-column PTY probes | Raw PTY is not an emulator; do not hang on DSR. |
| — | `FND-001` CI SHA refresh | Needs a linked run, not storage evidence. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked on remaining `G2` / `G3` / `G0` matrix |

## Goal

1. A foreign host uid (`nobody`, 65534) cannot list the `0700` store directory or
   read the `0600` database, WAL, or SHM while the owner writer is live.
2. The owner can still open and count the same store.
3. Mode bits stay `0700`/`0600` (P-1–P-4 remain green). No chown, no widen.
4. `G2` and `HIST-007` stay `validation` (write-ack budget still open).
   `PRM-002` stays `discovery` (wrap-column PTY probes remaining).

## Out of scope (hard)

- `seteuid` / `setuid` in the test process
- `unshare --map-user` of the same outer uid
- `useradd` / `adduser` / `apt install uidmap`
- `chown` or widening modes to make probes pass
- `#[ignore]` / skip-if-unavailable (not a G2 pass)
- Write-ack product optimization
- Changing `WRITER_BATCH_SIZE`, `wait_for_count`, ACK meaning, or MBX2
- Marking `G2` or `HIST-007` complete
- Committing, pushing, or editing shell startup files unless asked

## Method

Spawn `sudo -n -u nobody` as a child (real host uid 65534). Assert `sudo -n -u
nobody id -u` prints `65534` before store probes. Denied reads use `dd
if=PATH of=/dev/null bs=1 count=1`; directory denial uses `ls PATH`. Combined
stdout/stderr must not contain `PERM_SENTINEL` (`M-023`).

Keep the writer alive while probing WAL/SHM, same pattern as
`wal_and_shm_files_are_user_only`.

## Cases

| Case | What | Pass |
| --- | --- | --- |
| F-1 | Directory | After open+enqueue, dir mode `0700`; `sudo -n -u nobody ls` fails |
| F-2 | Database | Main db mode `0600`; foreign `dd` read fails; owner `count == 1` |
| F-3 | WAL and SHM | Sidecars exist at `0600` while writer is live; foreign `dd` fails |
| F-4 | Distinct host uid | `sudo -n -u nobody id -u` is `65534`, not the owner uid |

## Validation (recorded on Linux cloud / CI)

```bash
sudo -n -u nobody id
cargo test -p mbx --lib storage -- --nocapture foreign
cargo clippy --workspace --all-targets -- -D warnings
bash tests/run.bash
```

## Follow-on

- Remaining `G2`: write-ack budget only (do not chase without prompt-path proof)
- `G0`: platform matrix, `HRD-001` macOS PTY run, `PRM-004` representative percentiles
- `PRM-002`: wrap-column PTY probes from `RSH-004` baseline
