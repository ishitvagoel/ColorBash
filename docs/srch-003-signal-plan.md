# SRCH-003 leftover: search signal and terminal-state PTY (T-1–T-4)

Status: `complete` for this Strategy A signal/terminal-state slice (2026-08-17).
`SRCH-003` Strategy A close is `docs/srch-003-failed-filter-plan.md`. Overlay
and 100k interactive percentiles stay `deferred`. Interactive repo insert
stays unauthorized.

## Why this slice

Cwd empty-line and prefix/fuzzy filters are recorded. Overlay, ghost, and
highlighting stay blocked on after-every-key decoration (ADR 0003 / ADR 0009).
`HIST-010` / failed-command CLI live on other PRs. The named leftover this
branch can evidence is Ctrl+C, Ctrl+Z, resize, and `stty -g` around `\C-xh`
and `\C-xl`, matching editor `bind -x` (E-4, M-3, M-4) and foundation
terminal restoration.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Signal / terminal-state PTY (this plan) | Named `SRCH-003` leftover; no overlay. |
| — | Show age/cwd/status columns | Overlay unproven. |
| — | Repo / failed-command filters | Other PRs / `HIST-010`. |
| — | 100k interactive percentiles | `deferred` unless a functional defect. |
| — | Ghost / highlighting / popup | After-every-key leftover. |

## Goal

1. After `\C-xh` replaces `READLINE_LINE`, Ctrl+C cancels the line without
   executing the match. The next prompt runs a sentinel command.
2. `stty -g` before search insert, restore, Ctrl+C, and resize equals `stty -g`
   after those events. A sentinel still executes.
3. Resize after insert still submits the match; a follow-up command runs.
4. Ctrl+Z stops a foreground job, returns to the prompt, and `\C-xh` still
   inserts without executing until Enter.
5. Remaining Strategy A metadata leftover is opt-in failed insert
   (`docs/srch-003-failed-filter-plan.md`). Do not start overlay, ghost, or
   highlighting.

## Out of scope (hard)

- Overlay / age / status columns
- Rebinding `\C-r`, `\C-g`, Tab, or printables
- Repository or failed-command filters
- Logging command text (`M-023`)
- `set -euo pipefail` in sourced modules
- Marking `SRCH-003`, `COMP-004`, `COMP-005`, `GHST-*`, or `HLT-*` complete
  in this signal-only slice (`SRCH-003` Strategy A close is
  `docs/srch-003-failed-filter-plan.md`)
- Taking Readline ownership (ADR 0003)

## Asserts

| ID | Evidence |
| --- | --- |
| T-1 | Prefix insert then Ctrl+C: match does not execute; next prompt runs a sentinel |
| T-2 | `stty -g` identical before/after insert, `\C-xl` restore, Ctrl+C, and resize; sentinel runs |
| T-3 | Resize after insert still submits the match; follow-up command runs |
| T-4 | Ctrl+Z then empty-line `\C-xh` still inserts; Enter prints the sidecar match; a sentinel runs after job cleanup |

## Stop

Do not start overlay, ghost, or highlighting. `SRCH-003` Strategy A close is
`docs/srch-003-failed-filter-plan.md`.
