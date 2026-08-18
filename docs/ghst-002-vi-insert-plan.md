# GHST-002 leftover: vi-insert ghost wrapping (V-1–V-3)

Status: `complete` for vi-insert (2026-08-18). Dim paint and async lookup
remain. Do **not** mark `GHST-004` complete.

## Why this slice

Emacs printables, accept, and cycling are recorded. `set -o vi` starts a new
line in vi-insert; those keys were still stock `self-insert`, so ghost never
queried. EDT-001 already installs `bind -x` on vi-insert for the insert
chord; ghost should follow that keymap, not vi-command.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | vi-insert wrapping (this plan) | Named leftover after remaining printables. |
| — | Dim / highlighting | After-every-key paint still unproven. |
| — | vi-command | Letters are motion; do not wrap. |

## Goal

1. After a successful emacs install, also wrap ASCII printables on
   **vi-insert** with the same occupied-skip rules. Failure of the vi keymap
   must not undo emacs. Do not bind vi-command.
2. On vi-insert, `\C-m` / `\C-j` are stock `accept-line`. While a suffix is
   active, arm the same Readline-only kill-line + accept-line macro as emacs
   (M-041). Default helpers `\C-x\C-k` / `\C-x\C-m` are unbound pairs there.
   Do not bind `\ef` (ESC is `vi-movement-mode`). Word-accept uses stock
   `forward-word` CSI (`\e[1;5C`). Full accept uses stock `forward-char`
   (`\e[C` / `\eOC`). Skip `\C-f` (self-insert on vi-insert).
3. Default emacs-mode install still sets `_MBX_GHOST_VI_BOUND=1` because the
   vi-insert keymap is populated at source time (M-040).
4. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- vi-command wrapping
- Async generation IDs
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| V-1 | Default ghost+history install sets `_MBX_GHOST_VI_BOUND=1` (extend G-5) |
| V-2 | History+ghost PTY with `set -o vi`: record `echo MBX_GHST:alpha`; type `echo MBX_GHST:a`; the line shows the full command; Enter prints `MBX_GHST:a` not `MBX_GHST:alpha` |
| V-3 | Same vi setup; Right then Enter prints `MBX_GHST:alpha` |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-004` complete.
