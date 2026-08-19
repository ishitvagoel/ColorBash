# GHST-003 leftover: word-accept one suffix word (W-1–W-3)

Status: `complete` for word-accept (2026-08-17). Suggestion cycling is recorded
in `docs/ghst-003-cycle-plan.md`. Do **not** mark `GHST-004` complete.

## Why this slice

Full accept (Right / `\C-f`) is recorded in G-2. Fish-style ghost also accepts
the next word. That is stock `forward-word` with point still left of the
unaccepted suffix, so Enter's kill-line macro (M-041) still drops the rest.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Word-accept (this plan) | Named `GHST-003` leftover; no overlay. |
| — | Suggestion cycling | Recorded in `docs/ghst-003-cycle-plan.md`. |
| — | Dim / highlighting | After-every-key paint still unproven. |

## Goal

1. Wrap emacs `forward-word` on `\ef` and `\e[1;5C` when those keys are
   stock `forward-word`. Occupied keys are skipped.
2. With an active suffix, one word-accept moves `READLINE_POINT` to the end of
   the current alphanumeric word (skipping non-alnum first if needed) and
   records that as `_MBX_GHOST_POINT`. Remaining suffix stays after point.
3. A word-accept that reaches the end of the line is full accept (clear the
   suffix flag; disarm Enter). Enter still must not run unaccepted words.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| W-1 | Module: line `echo MBX_GHST:one two`, point on `n` of `one`; word-accept lands after `one`; `_MBX_GHOST_HAS` stays 1 |
| W-2 | Same setup; a second word-accept reaches the end and clears the suffix flag |
| W-3 | History+ghost PTY: record `echo MBX_GHST:one two`; type `echo MBX_GHST:o`; Alt-F then Enter prints `MBX_GHST:one` not `MBX_GHST:one two` |

## Stop

Do not start highlighting or a completion overlay. Do not mark `GHST-004`
complete. Cycling is specified in `docs/ghst-003-cycle-plan.md`.
