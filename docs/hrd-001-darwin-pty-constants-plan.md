# HRD-001 pre-work: Darwin PTY platform constants

Status: `complete` for D-1–D-3 (2026-08-16). Do not mark `G0`, `HRD-001`, `G2`,
or `HIST-007` complete. After this slice, remaining `G2` is still foreign-user
open and the write-ack budget. Full `HRD-001` macOS PTY matrix evidence still
requires a macOS host.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Darwin PTY constants cfg-split (this plan) | `crates/pty/src/sys.rs` copied Linux `O_CLOEXEC`, `poll` `nfds_t`, and `ptsname_r` onto all targets. Linux PTY tests can prove the Linux path did not regress; cited Darwin values unblock the macOS leg. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. This WSL user is uid 1000; `sudo -n` needs a password; `newuidmap` is missing; `unshare --map-user` still owns the file. Do not fake `seteuid`. Do not `apt install uidmap`. |
| 3 | Write-ack p95/p99 budget miss | W-1–W-4 prove samples exist at prompt return before SQLite drain. Do not chase product-code latency unless a test proves the prompt waits on SQLite, samples contain command text, or ACK waits for commit. |
| — | Full `HRD-001` macOS PTY matrix | Needs macOS CI or manual run after constants land. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-025` (platform FFI widths; no bad casts)
   and `M-014` (heading-anchored doc edits).
2. Read this file completely. Do not invent extra cases.
3. Read `crates/pty/src/sys.rs` (`O_NOCTTY` cfg split, `O_CLOEXEC`, `poll`,
   `ptsname_r`, `Termios` cfg).
4. Read `docs/roadmap.md` known-debt bullet and `HRD-001` row.
5. `git status --short`. Do not discard unrelated work.
6. Implement. Do not commit, push, or edit shell startup files unless asked.

## Goal

1. Linux keeps bit-identical `O_CLOEXEC`, `poll` `nfds_t` width, and `ptsname_r`
   behavior; `cargo test -p mbx-pty` and `bash tests/run.bash` stay green on
   this host.
2. macOS cfg splits cite Darwin header values for `O_CLOEXEC`, `poll` `nfds_t`,
   and `ptsname_r` (or a documented equivalent). `O_NOCTTY` and `Termios`/`TIOC*`
   splits stay as they are.
3. `HRD-001` and `G0` remain `not-started` / `validation`. Remaining `G2` stays
   foreign-user open and write-ack budget.

## Out of scope (hard)

- Running the full `HRD-001` pairwise Bash/OS/terminal matrix on macOS
- tmux, SSH, or platform-matrix percentile evidence
- Changing `Termios` / `TIOC*` splits beyond what already exists
- Foreign-user open, write-ack product-code optimization, history storage
- Marking `G0`, `HRD-001`, `G2`, or `HIST-007` complete
- Committing, pushing, or editing `~/.bashrc` unless the user asks

## Cases

| Case | What | Pass |
| --- | --- | --- |
| D-1 | Linux parity | `#[cfg(not(target_os = "macos"))]` keeps `O_CLOEXEC = 0o2000000`, `poll(..., nfds: u64, ...)`, and `ptsname_r` as today. Focused const assertions or tests prevent a bad Darwin cfg from changing Linux values. |
| D-2 | Darwin `O_CLOEXEC` | `#[cfg(target_os = "macos")]` sets `O_CLOEXEC = 0x01000000` with a comment citing Darwin `bsd/sys/fcntl.h` (`O_CLOEXEC` when `__DARWIN_C_LEVEL >= 200809L`). Do not copy the Linux octal onto macOS. |
| D-3 | Darwin `poll` and slave name | `poll` uses `nfds_t` = `unsigned int` per Darwin `bsd/sys/poll.h`. `ptsname_r` remains the bounded slave-name path (POSIX / macOS `grantpt(3)`; available macOS 10.13.4+). Buffer stays fixed at 128 bytes with NUL-termination check. |

## Sources

| Constant / API | Linux | Darwin |
| --- | --- | --- |
| `O_CLOEXEC` | `0o2000000` (`0x80000`) — glibc `fcntl.h` | `0x01000000` — [Darwin `fcntl.h`](https://github.com/apple/darwin-xnu/blob/main/bsd/sys/fcntl.h) |
| `poll` `nfds` | `unsigned long` / `u64` on x86_64 — POSIX/glibc | `unsigned int` — [Darwin `poll.h`](https://github.com/apple/darwin-xnu/blob/master/bsd/sys/poll.h) |
| `ptsname_r` | POSIX.1-2001 | macOS `grantpt(3)`; available 10.13.4+ |

## Follow-on

- Run `cargo test -p mbx-pty --test driver --test foundation` on macOS to prove
  the cfg-split path before marking any `HRD-001` evidence complete.
- Remaining `G0`: platform matrix, full `HRD-001` macOS leg, representative
  `PRM-004` percentiles.

## Validation (recorded on Linux/WSL)

```bash
cargo test -p mbx-pty
cargo test -p mbx-pty --test driver --test foundation
bash tests/run.bash
```
