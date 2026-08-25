# SRCH-003 leftover: cwd-scoped prefix and fuzzy search (C-5–C-8)

Status: `complete` for this Strategy A cwd-filter slice (2026-08-17).
`SRCH-003` Strategy A close is `docs/srch-003-failed-filter-plan.md`. Overlay
and 100k interactive percentiles stay `deferred`. Interactive repo insert
stays unauthorized.

## Why this slice

Empty-line search already prefers `start_cwd = $PWD`. Typed prefix and fuzzy
still used the global index, so a newer match from another directory won.
`HIST-008` already stores cwd. Optional `--cwd` on prefix/fuzzy is Strategy A
and does not draw metadata, steal `\C-r`, or duplicate `HIST-010`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Prefix/fuzzy cwd-first (this plan) | Named leftover after empty-line cwd. |
| — | Show age/cwd/status columns | Overlay unproven. |
| — | Repo / failed-command filters | Other PRs / `HIST-010`. |
| — | Signal / terminal-state PTY | Recorded in `docs/srch-003-signal-plan.md` (T-1–T-4). |
| — | 100k interactive percentiles | `deferred` unless a functional defect. |
| — | Ghost / highlighting / popup | After-every-key leftover. |

## Goal

1. `mbx history search prefix TEXT [--cwd PATH] [--limit N]` and
   `mbx history search fuzzy TEXT [--cwd PATH] [--limit N]`. `--cwd` is invalid
   on `recent` and `cwd`. PATH is a bound SQL argument, never interpolated.
2. Non-empty `\C-xh` when `MBX_SEARCH_CWD` is not `0` and `PWD` is set: prefix
   with `--cwd "$PWD"`, then fuzzy with `--cwd`, then global prefix, then
   global fuzzy. Empty-line behavior is unchanged.
3. `MBX_SEARCH_CWD=0` skips cwd-scoped prefix/fuzzy (global only).
4. CLI stdout remains command text only. Do not log command text or cwd
   (M-023). Remaining Strategy A metadata leftover is opt-in failed insert
   (`docs/srch-003-failed-filter-plan.md`).

## Out of scope (hard)

- Overlay / age / status columns
- Repository or failed-command filters
- Logging command text
- `set -euo pipefail` in sourced modules
- Stealing `\C-r` / `\C-g`
- Marking `SRCH-003` complete in this prefix-only slice (close is
  `docs/srch-003-failed-filter-plan.md`)

## Asserts

| ID | Evidence |
| --- | --- |
| C-5 | Two directories with the same prefix; newer other-cwd row exists; typed prefix in the older cwd inserts the cwd match |
| C-6 | Directory with no prefix/fuzzy cwd rows falls back to global prefix |
| C-7 | Module stub: prefix with cwd output wins; `MBX_SEARCH_CWD=0` uses global prefix |
| C-8 | Storage: `exact_prefix_in_cwd` / `fuzzy_in_cwd` omit other directories; CLI parses `--cwd` |

## Stop

Do not start overlay, ghost, or highlighting. `SRCH-003` Strategy A close is
`docs/srch-003-failed-filter-plan.md`.
