# GHST-002 leftover: Down / forward-history dismiss (D-1–D-2)

Status: `complete` for Down / forward-history (2026-08-20). Do **not** mark
`GHST-004` complete.

## Why this slice

Home / Up / backward-word are recorded. Down still leaves an active suffix in
`READLINE_LINE` while point moves, so Enter stays armed (M-041). Strip first,
then apply forward-history using the same bounded offset as Up.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Down / forward-history (this plan) | Named leftover after Home/Up. |
| — | Dim / highlighting | After-every-key paint still unproven. |
| — | Async stale-rejection | Needs IPC ADR (`GHST-001`). |

## Goal

1. Wrap stock `next-history` / CSI Down / `\C-n` on emacs and vi-insert where
   those keys are stock history motion (not `self-insert`). Occupied-skip.
2. While a suffix is active, strip with `_mbx_ghost_strip`. If
   `_MBX_GHOST_HIST_OFFSET > 0`, decrement and load that history row; if offset
   is already `0`, leave the stripped typed prefix (no older forward step).
3. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Async generation IDs (`GHST-001`)
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 or `GHST-004` complete

## Asserts

| ID | Evidence |
| --- | --- |
| D-1 | Module: after Up loads `echo MBX_GHST:beta` with offset 1; `_mbx_ghost_next_history` loads `echo MBX_GHST:alpha` or restores typed prefix and clears HAS |
| D-2 | History+ghost PTY: record `alpha` then `beta`; type `echo MBX_GHST:b`; Ctrl-P; Ctrl-N; Enter prints the restored row without the unaccepted typed-prefix suffix |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-004` complete.
