# HRD-001 Linux pairwise PTY (host-possible slice)

Status: `complete` for L-1–L-5 (2026-08-26). Linux is the satisfied `HRD-001`
slice for Strategy A MVP / `G5` close (`docs/g5-strategy-a-close-plan.md`).
The macOS leg is **`deferred`** (ADR 0012), not `blocked`. Overlay,
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
2. Record Linux evidence; macOS is `deferred` per ADR 0012 (not faked here).
3. Use `/usr/bin/tmux`, not the agent `exec-daemon` wrapper.

## Out of scope (hard)

- macOS / WSL hosts, Darwin PTY runs
- Overlay, highlighting, dim paint
- Percentile benches
- Real `sshd` round-trips (no daemon on this host)
- Package-manager installers
- Faking macOS PTY evidence on Linux
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

1. `docs/roadmap.md` — Linux L-1–L-5 recorded; macOS `deferred` (ADR 0012).
2. This file — Status `complete` after the tests are green.

## Remaining

macOS pairwise (`deferred`, ADR 0012). `HRD-003` deferred. Overlay deferred.
