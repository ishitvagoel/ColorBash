# COMP-004 leftover: ranked-cycle `bind -x` chords (C-1–C-6)

Status: `complete` (2026-08-25). Popup policy P-1–P-4 is recorded in
`docs/comp-004-popup-plan.md`. Ranked-accept A-1–A-6 is complete. Tab stays
stock. This packet adds optional next/prev chords that rotate the ranked
candidate list and replace the current word without drawing an overlay or
rebinding Tab. `COMP-005` Strategy A insert/fallthrough is closed separately
(`docs/comp-005-strategy-a-close-plan.md`). Do not mark `COMP-004` complete.

## Why this slice

Immediate next work after ranked-accept. `COMP-004` is in `discovery` with no
GUI overlay. Ranked-accept inserts only `_MBX_COMP_ORDER[0]`. The strongest
remaining `COMP-004` leftover that can produce evidence on this host is cycling
that ordered list with the same occupied-skip `bind -x` pattern. Do not treat
this as overlay navigation (ADR 0003).

This is a **small product slice**, same size as ranked-accept. Not a decoration
experiment.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Ranked-cycle chords (this plan) | Named `COMP-004` leftover; accept chord exists; wrap already fills `_MBX_COMP_ORDER`. |
| — | GUI overlay / arrow-key menu | Unproven; would need continuous decoration or Readline ownership (ADR 0003). |
| — | Ghost / highlighting / enhanced Ctrl+R | Still blocked on after-every-key decoration (B-5). |
| — | `HIST-010` / CLI metadata filters | Separate history slice; do not mix here. |
| — | Marking `COMP-004` complete | Overlay still unproven. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. After a wrapped completion run, snapshot the ranked candidate **strings** in
   `_MBX_COMP_RANKED_LIST` (first 64 entries of `_MBX_COMP_ORDER` mapped through
   `COMPREPLY`). `_MBX_COMP_RANKED_REPLY` remains the current head. `COMPREPLY`
   and Tab insertion bytes stay unchanged.
2. Install optional `bind -x` chords in emacs and vi-insert:
   - next: default `\C-xn` (`MBX_COMP_CYCLE_NEXT_KEYSEQ`)
   - prev: default `\C-xp` (`MBX_COMP_CYCLE_PREV_KEYSEQ`)
   Occupied-skip like accept (`MBX_COMP_CYCLE_OVERRIDE=1` to overwrite). Do not
   rebind Tab, arrows, or printable editing keys. Skip `bind -x` when `$-`
   lacks `i`. Do not reuse ghost `\C-x\C-n` / `\C-x\C-p` or search `\C-xh` /
   `\C-xl` (M-040).
3. Cycle replace rules (extend M-039; do not splice):
   - Empty snapshot or empty current word: no-op (do not rotate).
   - Current word is a **proper prefix** of `_MBX_COMP_RANKED_REPLY`: replace
     with the current head **without rotating** (same as first accept).
   - Current word **equals** `_MBX_COMP_RANKED_REPLY` and the list has at least
     two entries: rotate next (head → tail) or prev (tail → head), update
     `_MBX_COMP_RANKED_REPLY`, replace the word with the new head.
   - Otherwise (stale unrelated word): no-op (do not rotate).
4. Never insert kinds, descriptions, or scores. Never execute inserted text.
5. Clear `_MBX_COMP_RANKED_REPLY` and `_MBX_COMP_RANKED_LIST` at the next prompt.
6. `COMP-004` stays `discovery`. Do not mark `COMP-004` complete in this slice.

## Out of scope (hard)

- Reordering or mutating `COMPREPLY` for Tab insertion
- GUI overlay, candidate list rendering, or arrow-key navigation
- Rebinding Tab, arrows, or other printable keys (ADR 0003)
- Ghost, highlighting, enhanced Ctrl+R
- History / Git product code
- Changing MBX1 framing
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Percentile benches
- Marking `COMP-004` complete
- Committing unless asked
- Widening into overlay or continuous decoration

## Method

Reuse `_mbx_comp_accept_ranked` word-boundary replacement. Snapshot ranked
strings at wrap time so cycle does not depend on `COMPREPLY` surviving after
Tab. Prefix-only replacement cannot move `aaflag` → `zzflag`; equality with
the current ranked head is required for rotation.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| C-1 | Prefix cycle inserts head without rotating | Module: after R-2 wrap, cycle-next on `aa` → `aaflag`; list head stays `aaflag`. PTY: Tab + next chord + Enter → `\nGOT:aaflag\|`. | complete — `tests/bash/modules.bash`, `ranked_cycle_next_inserts_head_from_prefix` |
| C-2 | Equal head rotates to next | Module + PTY: from `aaflag`, cycle-next → `zzflag`. Tab + next + next + Enter → `\nGOT:zzflag\|`. Accept then next also yields `zzflag`. | complete — `ranked_cycle_next_rotates_from_accepted_head`, `ranked_cycle_after_accept_rotates_to_next` |
| C-3 | Prev rotates to last | Module: from `aaflag` with list `(aaflag, zzflag)`, cycle-prev → `zzflag`. | complete — `tests/bash/modules.bash`, `ranked_cycle_prev_wraps_to_last` |
| C-4 | Occupied keyseq skipped | PTY: pre-bind `\C-xn`; without override next is not bound; with `MBX_COMP_CYCLE_OVERRIDE=1` it is. | complete — `occupied_cycle_next_chord_is_not_overwritten`, `occupied_cycle_next_chord_override_installs` |
| C-5 | Stale unrelated word refused | Module + PTY: current word `ok` unchanged; list not rotated. | complete — `tests/bash/modules.bash`, `ranked_cycle_refuses_stale_unrelated_word` |
| C-6 | Metadata never inserted | PTY: cycle path output must not contain `EXTRA`. | complete — `ranked_cycle_metadata_never_inserted` |

If a measured result differs, record host bytes in the Status cell. Do not make
Tab follow `_MBX_COMP_ORDER`.

## Docs to update (this slice)

1. `docs/roadmap.md` — immediate next work; changelog row; `COMP-004` evidence
   note for C-1–C-6. Do not mark `COMP-004` complete.
2. `docs/architecture.md` — ranked-cycle chords; point at this plan.
3. `README.md` — tryable cycle chords.
4. This file — Status `ready` → `complete` after evidence.

## Validate

```bash
cargo test -p mbx-pty completion_harness -- --nocapture
bash tests/bash/modules.bash
bash tests/run.bash
```

## Remaining after this slice

`COMP-004` stays `discovery` until overlay strategy is resolved or scope is
explicitly narrowed in an ADR. GUI overlay remains unproven. Default cycle
chords are `\C-xn` / `\C-xp` so they do not collide with ghost or search.
`COMP-005` Strategy A insert/fallthrough is closed without overlay
(`docs/comp-005-strategy-a-close-plan.md`). `HRD-001` macOS matrix remains
separate.
