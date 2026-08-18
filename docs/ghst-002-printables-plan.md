# GHST-002 leftover: remaining printable self-insert (P-1–P-3)

Status: `complete` for remaining printables (2026-08-18). vi-insert is recorded
in `docs/ghst-002-vi-insert-plan.md`. Dim paint and async lookup remain. Do
**not** mark `GHST-004` complete.

## Why this slice

Letters, digits, space, and `_ - . : /` are wrapped. Other ASCII punctuation
is stock `self-insert` on this host, so typing `=` or `'` never queried the
sidecar. Occupied-skip still applies per key. Tab, `\C-r`, `\C-g`, `\C-x\C-r`,
and `\C-x\C-s` stay untouched.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Remaining printables (this plan) | Named ADR 0010 leftover; no overlay. |
| — | vi-insert keymap | Recorded in `docs/ghst-002-vi-insert-plan.md`. |
| — | Dim / highlighting | After-every-key paint still unproven. |

## Goal

1. Wrap every remaining ASCII printable that is stock `self-insert`, using
   Readline's quoted keyseq form so `"`, `\`, `$`, and `` ` `` bind safely.
2. Occupied non-`self-insert` keys are still skipped unless
   `MBX_GHOST_OVERRIDE=1`. Install still succeeds when at least one printable
   wraps (`_MBX_GHOST_BOUND=1`).
3. A prefix that includes `=` still shows a suffix; Enter runs the typed
   prefix. Do not steal Tab or search chords.
4. Do not mark `GHST-004` complete. Do not start highlighting, overlay, or
   vi-insert.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- vi-insert ghost wrapping
- Async generation IDs
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| P-1 | Module: quoted keyseq forms for `=`, `"`, `\`, and `\C-h`; insert `=` extends `echo foo` to `echo foo=bar` |
| P-2 | History+ghost PTY: record `echo MBX_GHST:foo=bar`; type `echo MBX_GHST:foo=`; the line shows the full command; Enter prints `MBX_GHST:foo=` not `MBX_GHST:foo=bar` |
| P-3 | Default install still sets `_MBX_GHOST_BOUND=1` (existing G-5) |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-004` complete.
vi-insert is specified in `docs/ghst-002-vi-insert-plan.md`.
