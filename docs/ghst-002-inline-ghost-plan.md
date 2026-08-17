# GHST-002 leftover: opt-in inline ghost suffix (G-1–G-6)

Status: `complete` for this Strategy A ghost slice (2026-08-17). Async lookup,
dim styling, word-accept, cycling, vi-insert, and remaining printables remain.
Do **not** mark `GHST-004` complete.

## Why this slice

The user asked for ghost. After-every-key ANSI decoration is still unproven.
ADR 0010 records a Readline-native substitute: keep the suggestion in
`READLINE_LINE` after `READLINE_POINT` so it appears to the right of the
cursor. `G2` permits history-driven editor experiments. Prefix search already
exists (`HIST-008`).

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Opt-in suffix ghost (this plan) | Named Phase 4 start; no overlay. |
| — | Async generation IDs (`GHST-001`) | Needs an IPC ADR; timeout-bounded sync is enough to show text. |
| — | Dim / highlighting / popup | Still need after-every-key paint. |
| — | Word-accept / cycling | After full accept. |
| — | `HIST-010` repo ranking | Other PRs. |

## Goal

1. Install only when `MBX_GHOST=1` and `MBX_HISTORY=1` in emacs. Skip occupied
   non-`self-insert` keys unless `MBX_GHOST_OVERRIDE=1`.
2. Typing at the end of the line may extend the buffer with one prefix match.
   Point stays on the typed prefix. Enter strips the suffix then `accept-line`.
3. Right / `\C-f` with an active suffix moves point to the end (full accept).
   Backspace strips, deletes one typed character, and refreshes.
4. Matches must be an exact byte prefix of the typed line, control-free, and
   bounded. Do not log command text (M-023). Do not execute the suffix.
5. Do not mark `GHST-001` or `GHST-004` complete. Do not start highlighting.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Rebinding Tab, `\C-r`, `\C-g`, `\C-j`, `\C-x\C-r`, `\C-x\C-s`
- `eval` / executing the line from `bind -x`
- `set -euo pipefail` in sourced modules
- Async MBX2 query protocol
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| G-1 | History+ghost PTY: record `echo MBX_GHST:alpha`; type `echo MBX_GHST:a`; the line shows the full command; Enter prints `MBX_GHST:a` not `MBX_GHST:alpha` |
| G-2 | Same setup; Right then Enter prints `MBX_GHST:alpha` |
| G-3 | `MBX_GHOST` unset: typing `echo MBX_GHST:a` then Enter prints `MBX_GHST:a`; the full sidecar row does not appear before Enter |
| G-4 | Missing helper: typing still inserts and Enter runs the typed bytes |
| G-5 | Default install with ghost+history sets `_MBX_GHOST_BOUND=1` |
| G-6 | Module stub: insert extends the suffix; strip restores the typed prefix; history-off insert does not query |

## Stop

Do not start highlighting or a completion overlay. Do not mark `GHST-004`
complete.
