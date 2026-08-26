# HRD-001 Linux pairwise PTY (host-possible slice)

Status: `complete` for L-1–L-5 (2026-08-26). The full `HRD-001` matrix stays
**blocked on macOS**. Do not mark `HRD-001` or `G5` complete. Overlay,
highlighting, dim paint, and percentile benches stay `deferred`.

## Why this slice

Immediate next work after merging `HRD-002` / `HRD-004`. The remaining
`HRD-001` macOS leg cannot run on this Linux host. G5 still names tmux, SSH
context, nested shells, login shells, and fullscreen programs. Those can
produce Linux PTY evidence now without faking Darwin.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Linux pairwise PTY (this plan) | Nested, SSH flag, login, vim restore, tmux are runnable here. |
| — | macOS pairwise matrix | Needs a macOS host. Darwin constants D-1–D-3 already recorded. |
| — | Overlay / highlighting / dim paint | ADR 0003; G5 revisit. |
| — | `HRD-003` percentiles | `deferred`. |

## Goal

1. Record Linux PTY evidence for nested interactive Bash, SSH context in the
   live prompt, login-shell install, vim fullscreen then restore, and tmux.
2. Keep `HRD-001` `blocked` (macOS leftover). Do not mark `G5` complete.
3. Use `/usr/bin/tmux`, not the agent `exec-daemon` wrapper.

## Out of scope (hard)

- macOS / WSL hosts, Darwin PTY runs
- Overlay, highlighting, dim paint
- Percentile benches
- Real `sshd` round-trips (no daemon on this host)
- Package-manager installers
- Marking `HRD-001` or `G5` complete
- `set -euo pipefail` or `MBX_DBG` in sourced modules

## Asserts

| ID | Evidence |
| --- | --- |
| L-1 | Nested `bash --noprofile --norc -i`; inner marker; `exit`; outer MBX prompt still runs a command |
| L-2 | `SSH_CONNECTION` + `HOSTNAME=testhost` live prompt contains `ssh: testhost`; a command still runs |
| L-3 | Isolated `HOME` login shell (`bash -il`) sources `.bash_profile` → `init.bash`; marker command runs |
| L-4 | `vim -u NONE -n` then `:q!`; `stty -g` matches pre-vim; next command runs |
| L-5 | `/usr/bin/tmux` unique socket; inner MBX prompt; marker command; server gone after session drop |

## Docs to update

1. `docs/roadmap.md` — `HRD-001` stays `blocked`; note Linux L-1–L-5. Changelog.
   Immediate next work remains macOS. Do not mark `G5` complete.
2. This file — Status `complete` after the tests are green.

## Remaining

macOS pairwise. `HRD-003` deferred. Overlay deferred.
