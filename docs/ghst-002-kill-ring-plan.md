# GHST-002 leftover: kill-ring isolation (K-1–K-3)

Status: `complete` for kill-ring isolation (2026-08-19). Do **not** mark
`GHST-004` complete.

## Why this slice

Home / Up / backward-word dismiss is recorded. Enter still arms a Readline macro
that uses reserved `kill-line` from point, which copies the unaccepted suffix
onto the kill ring. Motion dismiss already uses `_mbx_ghost_strip` (parameter
expansion). Enter should discard the suffix the same way: repeated stock
`delete-char` (`\C-d`) from point, then reserved `accept-line` (M-041).

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Kill-ring isolation (this plan) | Named ADR 0010 leftover after motion dismiss. |
| — | Dim / highlighting | After-every-key paint still unproven. |

## Goal

1. While a suffix is active, arm Enter (`\C-m` / `\C-j` when stock
   `accept-line`) with a Readline-only macro: `N` × reserved `delete-char`
   (default `\C-x\C-d`, bounded by suffix length / 256), then reserved
   `accept-line` (default `\C-x\C-m`). No `kill-line`, no bind -x, no `eval`.
2. Rebuild the macro whenever `_mbx_ghost_show` changes the suffix (including
   cycle next/prev with different row lengths). Disarm before re-arm.
3. Drop the `MBX_GHOST_KILL_KEYSEQ` install prerequisite. Bind the reserved
   delete helper on emacs and vi-insert (`\C-d` is `vi-eof-maybe` in vi-insert).
   `MBX_GHOST_ACCEPT_KEYSEQ` and optional `MBX_GHOST_DELETE_KEYSEQ` remain.
4. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Async stale rejection (`GHST-001`)
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| K-1 | Module: `_mbx_ghost_enter_delete_macro 4` yields four reserved delete-char steps plus the configured accept keyseq; no `kill-line` substring |
| K-2 | History+ghost PTY: record `echo MBX_GHST:alpha`; type `echo MBX_GHST:a`; Enter prints `MBX_GHST:a`; Ctrl-Y at the next prompt does not paste `pha` |
| K-3 | Module: after `_mbx_ghost_show` with suffix length 4, `_MBX_GHOST_ENTER_ARMED=1`; a second show with suffix length 2 rebuilds the armed macro (disarm then arm) |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-004` complete.
