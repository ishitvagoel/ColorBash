# GHST-002 leftover: Home / Up / backward-word dismiss (H-1–U-2)

Status: `complete` for Home, Up, and backward-word (2026-08-19). Kill-ring
isolation is `docs/ghst-002-kill-ring-plan.md`. Do **not** mark `GHST-004` complete.

## Why this slice

Left dismiss is recorded. Home, Up, and backward-word still leave an active
suffix in `READLINE_LINE` while point moves, so Enter stays armed (M-041) and
can delete typed bytes or submit the wrong line. Strip first (parameter
expansion, not `kill-line`), then apply the stock motion.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Home / Up / backward-word (this plan) | Named ADR 0010 leftover after Left. |
| — | Kill-ring isolation | Recorded in `docs/ghst-002-kill-ring-plan.md`. |
| — | Dim / highlighting | After-every-key paint still unproven. |

## Goal

1. Wrap stock `beginning-of-line`, `previous-history`, and `backward-word` on
   emacs and vi-insert where those keys are not `self-insert`. Occupied-skip.
   Do not bind vi-insert `\C-a` (`self-insert`) or emacs `\ef` occupied paths
   already used for word-accept.
2. While a suffix is active, strip with `_mbx_ghost_strip`, then move point or
   load history. Up uses bounded `fc` / `history` reads (no command-text
   logging). Reset the history offset on new typing.
3. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Down / forward-history
- Replacing Enter `kill-line` (kill-ring isolation)
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| H-1 | Module: active suffix `echo MBX_GHST:alpha` at prefix point; `_mbx_ghost_beginning` restores `echo MBX_GHST:a`, point 0, `_MBX_GHOST_HAS=0` |
| W-4 | Module: active suffix on `echo MBX_GHST:one two` at end of `one`; `_mbx_ghost_backward_word` restores typed prefix and lands before `two` |
| U-2 | History+ghost PTY: record `echo MBX_GHST:alpha` then `echo MBX_GHST:beta`; type `echo MBX_GHST:b`; Ctrl-P loads `echo MBX_GHST:beta`; Enter prints `MBX_GHST:beta` not the unaccepted suffix from the typed prefix |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-004` complete.
Do not start kill-ring isolation in this slice.
