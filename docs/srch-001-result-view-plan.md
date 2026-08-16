# SRCH-001 leftover: bounded match cycling (V-1–V-4)

Status: `complete` for the Strategy A result view (2026-08-16). The explicit
insert chord already exists (S-1–S-7, ADR 0009). This packet snapshots a
bounded sidecar result list and cycles it with the same `\C-xh` chord. It does
**not** draw an overlay, show age/cwd/status columns, restore the original
buffer on cancel, or steal stock `\C-r`.

## Why this slice

Immediate next work after the insert action. `SRCH-001` stays incomplete until
a result view exists. A metadata list UI would need after-every-key decoration
or printable rebinds. Repeating the existing chord is Strategy A and matches
ranked-accept's "explicit action, Readline owns the buffer" pattern.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Match cycling (this plan) | Named `SRCH-001` leftover; stays Strategy A. |
| — | Cancel restoration (`SRCH-002`) | Needs a restore action or search mode; separate slice. |
| — | Age/cwd/status columns / overlay | Overlay unproven; metadata filters are `SRCH-003`. |
| — | Ghost / highlighting / popup | Still blocked on after-every-key decoration. |
| — | `HIST-010` / `GIT-003` | Separate leftover; do not mix. |

## Goal

1. First `\C-xh` queries a bounded list (`MBX_SEARCH_LIMIT`, default 8, max 16):
   empty line → `recent`; non-empty → prefix, then fuzzy if prefix is empty.
   Insert the first line. Never execute.
2. Repeating the chord while `READLINE_LINE` equals the current snapshot entry
   advances to the next match and wraps. Editing the line starts a new query.
3. Clear the snapshot at the next prompt (`_mbx_render_prompt`), same lifetime
   as `_MBX_COMP_RANKED_REPLY` (M-039 analog).
4. Do not use `_mbx_read_bounded_response` for the match list (M-041).
5. Mark `SRCH-001` complete when V-1–V-4 have evidence. Do not mark `SRCH-002`,
   `SRCH-003`, `COMP-004`, or `COMP-005` complete. Do not start ghost or overlay.

## Out of scope (hard)

- Overlay, age/cwd/status columns, printable-key rebinds
- Dedicated cancel/restore chord (`SRCH-002`)
- Repository / failed-command filters
- Changing MBX1 framing
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Stealing `\C-r` / `\C-x\C-r` / `\C-x\C-s`
- Ghost, highlighting, completion popup

## Asserts

| ID | Evidence |
| --- | --- |
| V-1 | Two prefix matches: first chord inserts newest, second chord inserts older; Enter executes the older line; neither chord executes |
| V-2 | Empty-line recent: first chord newest, second chord previous; Enter executes the previous |
| V-3 | Module stub with two lines: insert, cycle, wrap back to the first |
| V-4 | After Enter, `${#_MBX_SEARCH_MATCHES[@]}` is 0 at the next prompt |

## Stop

Do not start `SRCH-002`, ghost, or overlay.
