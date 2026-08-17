# SRCH-002 leftover: cancel restoration (R-1–R-4)

Status: `complete` for Strategy A restore (2026-08-17). Exact insert without
execution already has S-1 evidence. This packet restores the pre-search
`READLINE_LINE` with a dedicated `bind -x` chord. It does **not** draw an
overlay, rebind printables, steal stock `\C-r` / `\C-g`, or start `SRCH-003`.

## Why this slice

Immediate next work after bounded cycling. `SRCH-002` named leftover is cancel
restoration. Wrapping the cycle back to the original line is optional; a
dedicated restore chord is the explicit cancel action and stays Strategy A
(ADR 0009).

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Restore chord (this plan) | Named `SRCH-002` leftover; no printable rebinds. |
| — | Metadata overlay / 100k interactive (`SRCH-003`) | After restore; repo filters also need `HIST-010`. |
| — | Ghost / highlighting / popup | Still blocked on after-every-key decoration. |
| — | `HIST-010` / `GIT-003` | Separate leftover; do not mix. |

## Goal

1. On a **new** search that inserts a match, snapshot `_MBX_SEARCH_ORIGINAL`
   and `_MBX_SEARCH_ORIGINAL_POINT` from the typed line (including empty).
   Cycling must not overwrite that snapshot.
2. Default restore chord `\C-xl` (Ctrl-X then `l`; unbound in stock emacs and
   vi-insert; M-040). Do not steal `\C-g`, `\C-x\C-g`, `\C-r`, `\C-x\C-r`,
   `\C-x\C-s`, `\C-x\C-y`, or `\C-x\C-a`. Occupied restore keyseqs are skipped
   unless `MBX_SEARCH_RESTORE_OVERRIDE=1`. `MBX_SEARCH_RESTORE_KEYSEQ` may
   select another sequence. Skip restore bind when it equals the insert
   keyseq.
3. `_mbx_search_restore` requires `MBX_HISTORY=1` and a saved original.
   It writes `READLINE_LINE` / `READLINE_POINT` back, clears the snapshot, and
   never executes. No snapshot → no-op. History off → no-op. Failed new query
   clears any stale original.
4. `_mbx_search_clear` (next prompt) drops original and matches together.
5. Mark `SRCH-002` complete when R-1–R-4 have evidence. Do not mark
   `SRCH-003`, `COMP-004`, or `COMP-005` complete. Do not start ghost or overlay.

## Out of scope (hard)

- Overlay, age/cwd/status columns, printable-key rebinds
- Wrapping the cycle past the last match onto the original line
- Repository / failed-command filters
- Changing MBX1 framing
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Stealing `\C-r` / `\C-x\C-r` / `\C-x\C-s` / `\C-g` / `\C-x\C-g`
- Ghost, highlighting, completion popup

## Asserts

| ID | Evidence |
| --- | --- |
| R-1 | Prefix insert then restore: Enter executes the typed prefix (`MBX_SRCH:a\n`), not the sidecar match; neither chord executes. Cycle then restore still restores the typed prefix. |
| R-2 | Restore with no snapshot is a no-op (typed line executes unchanged). |
| R-3 | Occupied `\C-xl` is not overwritten; `_MBX_SEARCH_RESTORE_BOUND=0`. |
| R-4 | Default install on stock emacs sets `_MBX_SEARCH_RESTORE_BOUND=1`. |

Module contracts: insert then restore returns the typed query (including empty);
restore without a snapshot is a no-op; history-off restore is a no-op; a failed
search does not revive a stale original.

## Stop

Do not start `SRCH-003`, ghost, or overlay.
