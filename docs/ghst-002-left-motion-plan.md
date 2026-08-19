# GHST-002 leftover: Left dismisses an unaccepted suffix (L-1–L-3)

Status: `complete` for Left / backward-char (2026-08-18). Home, Up, and
backward-word are recorded in `docs/ghst-002-home-up-motion-plan.md`.
Kill-ring isolation is recorded in `docs/ghst-002-kill-ring-plan.md`. Do **not**
mark `GHST-004` complete.

## Why this slice

Printables, accept, cycling, and vi-insert are recorded. Left is still stock
`backward-char`, so an active suffix stays in `READLINE_LINE` while point moves
into the typed prefix. Enter stays armed (M-041) and `kill-line` from the new
point deletes typed characters. Wrapping Left to strip first (parameter
expansion, not `kill-line`) then `backward-char` dismisses the suggestion
without executing it.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Left / `\C-b` strip (this plan) | Named ADR 0010 leftover; no overlay. |
| — | Home / Up / backward-word | Same class of unwrapped motion; later leftover. |
| — | Kill-ring isolation | Recorded in `docs/ghst-002-kill-ring-plan.md`. |
| — | Dim / highlighting | After-every-key paint still unproven. |

## Goal

1. Wrap stock `backward-char` on emacs (`\e[D`, `\C-b`, `\eOD`) and vi-insert
   (`\e[D`, `\eOD` only). Occupied-skip. Do not wrap vi-insert `\C-b`
   (`self-insert`). Do not wrap vi-command.
2. While a suffix is active, the wrapper strips with `_mbx_ghost_strip` (no
   kill-ring) then moves point one character left. Enter is disarmed, so the
   typed prefix is accepted unchanged.
3. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Home / `\C-a`, Up / Down, `\eb` / Ctrl-Left
- Replacing Enter `kill-line` (kill-ring isolation)
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| L-1 | Module: active suffix `echo MBX_GHST:alpha` at prefix point; `_mbx_ghost_backward` restores `echo MBX_GHST:a`, point 14, `_MBX_GHOST_HAS=0` |
| L-2 | History+ghost PTY: record `echo MBX_GHST:alpha`; type `echo MBX_GHST:a`; Left then Enter prints `MBX_GHST:a` not `MBX_GHST:alpha` and not `MBX_GHST:` (unwrapped Left plus armed `kill-line` would drop the last typed character) |
| L-3 | Default install still sets `_MBX_GHOST_BOUND=1` (existing G-5) |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-004` complete.
Do not start Home / kill-ring isolation in this slice.
