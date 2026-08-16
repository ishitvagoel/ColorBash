# G3 decision: editor-integration evidence inventory

Status: `validation` (2026-08-16). `EDT-001` and `G3` are in `validation`. This
packet records which G3 bullets have PTY evidence and keeps continuous
decoration unproven. Do not mark `G3` or `EDT-001` complete.

Do not start ghost, popup, highlighting, or `COMP-003`.

## Why this slice

Immediate next work after the `G4` inventory. Every named G3 insert /
keymap / paste / resize / Ctrl+Z bullet now has host PTY bytes. The
remaining G3 gap is continuous after-every-key decoration (B-5). ADR 0003
keeps Readline ownership; that gap is not a Composer feature slice.

This is a **decision / inventory** slice, not a new editor case. Keep the
same size as `docs/g4-decision-plan.md`. Do not widen into `COMP-003` or
ghost.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | G3 evidence inventory (this plan) | Gate is still `validation` after functional evidence landed. |
| 2 | Continuous decoration / after-every-key redraw | Unproven. Needs a new experiment or ADR 0003 revisit. |
| — | `COMP-003` ranking / popup | Blocked until `G4` is `complete`. |
| — | Ghost / highlighting / Ctrl+R UI | Blocked on `G3` complete. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Map each G3 pass bullet to existing tests. Do not invent new cases.
2. Record continuous decoration as unproven (B-5). Do not write a new ADR
   unless evidence already shows printable-key rebinding is required (it
   does not).
3. Keep `G3` and `EDT-001` in `validation`. Do not mark either `complete`.
4. Immediate next work after this slice: do not start popup, ghost, or
   `COMP-003`. `PRM-006` duration policy remains a later discovery leftover.

## Out of scope (hard)

- New PTY cases or product changes in `bash/editor.bash`
- Rebinding printable keys or taking Readline ownership (ADR 0003)
- Ghost, completion popup, syntax highlighting, `COMP-003`
- Measuring or chasing percentiles
- Marking `G3`, `G4`, `EDT-001`, `COMP-001`, or `COMP-002` complete
- `set -euo pipefail` or `MBX_DBG`
- Committing unless asked
- Creating a second plan file
- Widening this packet into two gates or a feature bundle

## Evidence inventory (do not add tests)

| G3 bullet | Evidence | Status |
| --- | --- | --- |
| `READLINE_LINE` / `READLINE_POINT` insert without execute | E-1–E-4 in `docs/edt-001-bind-x-plan.md` | recorded |
| Exact bytes, cursor, suffixes, quoting, multiline | B-1–B-4 in `docs/edt-001-exact-bytes-plan.md` | recorded |
| Do not overwrite unknown bindings without opt-in | E-2 (`MBX_EDITOR_OVERRIDE=1`) | recorded |
| emacs + vi-insert, bracketed paste, resize, Ctrl+Z | M-1–M-4 in `docs/edt-001-g3-matrix-plan.md` | recorded |
| Insert-time Readline redraw without printable rebinds | B-5 note in `docs/edt-001-exact-bytes-plan.md` | recorded |
| Continuous after-every-key decoration | Unproven. Ghost / highlighting / popup stay blocked. | unproven |

Do not rewrite those case IDs or expected GOT lines.

## Docs to update (this slice)

1. `docs/roadmap.md` — keep `G3` `validation`. Immediate next work: do not
   start ghost / popup / `COMP-003`; continuous decoration stays unproven.
   Changelog row. Do not mark `G3` complete.
2. This file — Status `ready` → `validation` after the roadmap edit lands.

## Remaining after this slice

`G3` is `validation`, not `complete`. Ghost, highlighting, and enhanced
Ctrl+R stay blocked. `COMP-003` stays blocked on `G4` complete. The 5 ms
leftover stays `deferred`. `HRD-001` still needs a macOS host.
`PRM-006` duration policy is specified in `docs/prm-006-duration-plan.md`.
