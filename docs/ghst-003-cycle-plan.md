# GHST-003 leftover: cycle prefix matches (C-1–C-3)

Status: `complete` for cycling (2026-08-17). Full and word accept were already
recorded. Do **not** mark `GHST-004` complete.

## Why this slice

Right / `\C-f` full accept and `\ef` / Ctrl-Right word-accept are recorded.
Ghost still showed only `--limit 1`. Prefix search can return several rows;
cycling them needs no overlay or dim paint.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Suggestion cycling (this plan) | Named `GHST-003` leftover; no overlay. |
| — | Dim / highlighting | After-every-key paint still unproven. |
| — | Async lookup (`GHST-001`) | Needs an IPC ADR. |

## Goal

1. Query `history search prefix` with `--limit` 8 (override `MBX_GHOST_LIMIT`,
   clamped 1–8). Collect unique exact-prefix, control-free matches with a
   nonempty suffix ≤ 256 bytes. Show the newest first.
2. Store the typed prefix length at refresh. Cycle restores that point even
   after word-accept, replacing the displayed candidate.
3. Wrap next/prev on stock-unbound `\C-x\C-n` / `\C-x\C-p`. Occupied cycle
   keys are skipped and do not abort install. Do not use `\en` / `\ep` /
   `\e/` (M-040).
4. Cycle is a no-op unless a suffix is active and at least two candidates
   exist. Enter still runs only the typed prefix until the user accepts.
5. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Async generation IDs / stale-result rejection
- Rebinding Tab, `\C-r`, `\C-g`, `\C-x\C-r`, `\C-x\C-s`
- `eval` from bind -x
- Remaining printables / vi-insert
- Marking Phase 4 complete

## Asserts

| ID | Evidence |
| --- | --- |
| C-1 | Module: stub prints two prefix rows; insert shows the first; cycle next shows the second; cycle next wraps; cycle prev wraps |
| C-2 | History+ghost PTY: record `echo MBX_GHST:one` then `echo MBX_GHST:two`; type `echo MBX_GHST:`; the line shows `two`; Ctrl-X Ctrl-N shows `one` |
| C-3 | Same setup after cycle: Right then Enter prints `MBX_GHST:one` not `MBX_GHST:two`; sidecar admits the older command |

## Stop

Do not start highlighting, overlay, or async IPC. Do not mark `GHST-004`
complete.
