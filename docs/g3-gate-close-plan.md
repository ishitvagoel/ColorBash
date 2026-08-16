# G3 gate-close: editor-integration gate decision

Status: `complete` (2026-08-16). `G3` gate closed. `G4` and `COMP-003` are
`complete`. This packet closes the `G3` gate: explicit `bind -x` insert /
keymap / paste / resize / Ctrl+Z evidence is complete; continuous
after-every-key decoration stays unproven and blocks ghost / highlighting.
Do not start popup, ghost, or `COMP-004`. Do not rebind printable keys.

## Why this slice

Immediate next work after `COMP-003`. The `G3` evidence inventory
(`docs/g3-decision-plan.md`) already maps every functional insert bullet to
PTY tests. The remaining G3 bullet is continuous after-every-key decoration
(B-5). ADR 0003 keeps Readline ownership and says true continuous decoration
and GUI-like popups may be delayed. G3's pass line is to **demonstrate**
whether Readline augmentation can meet redraw needs without printable-key
rebinds — B-5 already records that insert-time redraw works and continuous
decoration does not.

This is a **gate-close decision** slice, same size as
`docs/g4-gate-close-plan.md`. Not a new editor case and not `COMP-004`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `G3` gate close (this plan) | Functional `bind -x` evidence complete; continuous decoration recorded unproven. |
| 2 | Continuous-decoration experiment | Unproven. Would rebind printables or revisit ADR 0003. Not this packet. |
| — | Popup / `COMP-004` | Unblock for planning only after `G3` closes; separate slice. |
| — | Ghost / highlighting / Ctrl+R UI | Still need after-every-key or an accepted redraw ADR. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Reconfirm each functional `G3` pass bullet maps to existing tests. Do not
   invent new cases.
2. Record that unproven continuous decoration does **not** block `G3` gate
   close for explicit `bind -x` actions (cite B-5 and ADR 0003). Ghost and
   highlighting stay blocked on that leftover, not on G3 status.
3. Move `G3` from `validation` to `complete`. `EDT-001` may move to
   `complete` (prototype exit met). Do not mark `COMP-001` or `COMP-002`
   complete.
4. Immediate next work: do not start `COMP-004`, popup, or ghost in this
   slice.

## Out of scope (hard)

- New PTY cases or product changes in `bash/editor.bash`
- Rebinding printable keys or taking Readline ownership (ADR 0003)
- Starting `COMP-004`, popup, ghost, highlighting, or enhanced Ctrl+R
- Measuring or chasing percentiles
- Marking `COMP-001` or `COMP-002` complete
- `set -euo pipefail` or `MBX_DBG`
- Committing unless asked
- Creating a second plan file
- Widening into popup or a decoration experiment

## Evidence inventory (do not add tests)

Reconfirm rows from `docs/g3-decision-plan.md`. Do not rewrite case IDs.

| G3 bullet | Evidence | Gate-close status |
| --- | --- | --- |
| `READLINE_LINE` / `READLINE_POINT` insert without execute | E-1–E-4 | satisfied |
| Exact bytes, cursor, suffixes, quoting, multiline | B-1–B-4 | satisfied |
| Do not overwrite unknown bindings without opt-in | E-2 | satisfied |
| emacs + vi-insert, bracketed paste, resize, Ctrl+Z | M-1–M-4 | satisfied |
| Insert-time Readline redraw without printable rebinds | B-5 note | satisfied |
| Continuous after-every-key decoration | Unproven. Blocks ghost / highlighting. | unproven — does not block gate close |

## Method

Read `docs/g3-decision-plan.md`, `docs/edt-001-exact-bytes-plan.md` B-5,
ADR 0003, and `docs/roadmap.md` G3 exit criteria. Confirm functional bullets
against `crates/pty/tests/editor_bind_x.rs` — do not add PTY cases unless a
named row has no evidence (none expected).

Product code only if a test proves a functional `G3` bullet fails today. The
expected outcome is **no product change** — docs and gate status only.

## Docs to update (this slice)

1. `docs/roadmap.md` — `G3` `validation` → `complete`. `EDT-001` → `complete`
   if prototype exit is met. Phase 4 / 6 / 8: ghost / highlighting / Ctrl+R
   stay blocked on unproven continuous decoration. `COMP-004` unblocked for
   planning only. Immediate next work: do not start popup / ghost.
   Changelog row.
2. `docs/g3-decision-plan.md` — short “Gate closed” note pointing at this
   file; do not change case IDs.
3. `docs/edt-001-exact-bytes-plan.md` remaining line — G3 closed; continuous
   decoration leftover still blocks ghost / highlighting.
4. This file — Status `ready` → `complete` after the roadmap edit lands.

## Remaining after this slice

`G3` is `complete`. Continuous decoration stays unproven and still blocks
ghost and highlighting. `COMP-004` may be planned next but is not started
here. The 5 ms leftover is reviewed before `G5`. `HRD-001` still needs a
macOS host.
