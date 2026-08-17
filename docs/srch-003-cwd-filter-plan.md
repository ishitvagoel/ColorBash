# SRCH-003 leftover: cwd-scoped empty-line search (C-1–C-4)

Status: `complete` for this Strategy A cwd-filter slice (2026-08-17). Age/status
overlay, 100k interactive latency, repo filters, failed-command filters, and
signal/terminal-state evidence remain. Do **not** mark `SRCH-003` complete.

## Why this slice

`SRCH-002` restore is done. Overlay and ghost stay blocked. Repository filters
need `HIST-010` (other PRs). Failed-command CLI search is a separate PR. Cwd is
already indexed (`HIST-008`, `mbx history search cwd`). Empty-line `\C-xh`
currently uses global `recent`, so a newer command from another directory wins.
Preferring `start_cwd = $PWD` is Strategy A metadata filtering without drawing
cwd on screen.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Empty-line cwd-first (this plan) | Uses existing cwd query; no overlay. |
| — | Prefix/fuzzy also cwd-scoped | Recorded in `docs/srch-003-cwd-prefix-plan.md` (C-5–C-8). |
| — | Show age/cwd/status columns | Overlay unproven. |
| — | Repo / failed-command filters | Other PRs / `HIST-010`. |
| — | 100k interactive percentiles | `deferred` unless a functional defect. |
| — | Ghost / highlighting / popup | After-every-key leftover. |

## Goal

1. Empty `READLINE_LINE` + insert chord: if `MBX_SEARCH_CWD` is not `0` and
   `PWD` is non-empty, query `history search cwd "$PWD"` with the same bound as
   recent. If that returns at least one line, that is the snapshot. Otherwise
   fall back to `history search recent`.
2. `PWD` is untrusted display data: pass it as a helper argument only (SQL bind
   on the CLI side). Do not interpolate it into `PS1` or traces (M-023).
3. Prefix and fuzzy stay global. Do not rebind printables. Do not steal `\C-r`.
4. `MBX_SEARCH_CWD=0` restores global-recent empty-line behavior.
5. Do not mark `SRCH-003` complete. Do not start overlay, ghost, highlighting,
   `HIST-010`, or failed-command CLI.

## Out of scope (hard)

- Overlay / age / status columns
- Combined prefix+cwd or fuzzy+cwd SQL
- Repository or failed-command filters
- Logging command text or cwd traces
- `set -euo pipefail` in sourced modules
- Stealing `\C-r` / `\C-g`
- Marking `SRCH-003` complete

## Asserts

| ID | Evidence |
| --- | --- |
| C-1 | Two directories: newer other-cwd command exists; empty chord in the older cwd inserts the cwd match, not the global newest; neither chord executes |
| C-2 | Empty directory (no cwd rows) falls back to global recent |
| C-3 | Module stub: empty line with cwd output uses cwd; `MBX_SEARCH_CWD=0` uses recent |
| C-4 | Existing same-cwd empty-line S-2 / V-2 still insert newest in that directory |

## Stop

Do not start overlay, ghost, `HIST-010`, or failed-command search. Do not mark
`SRCH-003` complete.
