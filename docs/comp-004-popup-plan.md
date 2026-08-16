# COMP-004 slice: completion-popup policy decision

Status: `discovery` (2026-08-16). `G3` / `G4` / `COMP-003` are `complete`. This
packet records the popup decision: do not build a GUI overlay. Tab stays
stock. Continuous decoration stays unproven. Do not start ghost or
highlighting. Do not rebind printable keys or Tab.

## Why this slice

Immediate next work after `G3` gate close. `COMP-004` is `not-started` and
unblocked for planning. ADR 0003 already says true continuous decoration and
GUI-like popups may be delayed. A full overlay would need after-every-key
redraw or Readline ownership — the same unproven leftover that blocks ghost
and highlighting.

This is a **decision / inventory** slice, same size as
`docs/g3-gate-close-plan.md`. Not a rendering experiment and not `HIST-009`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Popup policy decision (this plan) | Named leftover; overlay is unproven without a decoration hook. |
| 2 | Ranked-accept `bind -x` chord | Later `COMP-004` leftover. Needs this policy first. |
| — | Overlay menu / keyboard navigation | Unproven. Would rebind Tab or printables or take Readline ownership. |
| — | Ghost / highlighting / Ctrl+R UI | Still need after-every-key or an accepted redraw ADR. |
| — | `HIST-009` fuzzy history | UI-free; do not mix into this packet. |
| — | `GIT-004` Git kinds | Separate leftover after COMP-003. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Map each popup claim to existing evidence. Do not invent overlay cases.
2. Decide: no GUI overlay in this milestone. Tab remains stock Bash
   insertion (`COMPREPLY` order). Ranked scores stay additive
   (`_MBX_COMP_ORDER`).
3. Record a later leftover: optional explicit `bind -x` chord that inserts
   the top-ranked candidate without changing Tab. Do not implement it here.
4. Move `COMP-004` from `not-started` to `discovery` (overlay strategy
   unresolved). Do not mark it `complete`. `G3` stays `complete`.

## Out of scope (hard)

- Drawing a menu, overlay, or candidate list in the terminal
- Rebinding Tab, arrows, or printable keys
- Taking Readline ownership (ADR 0003)
- Implementing the ranked-accept chord
- Ghost, highlighting, enhanced Ctrl+R
- `HIST-009` / `GIT-004` product code
- Changing MBX1 framing
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG`
- Percentile benches
- Marking `COMP-004`, `COMP-001`, or `COMP-002` complete
- Committing unless asked
- Creating a second plan file
- Widening into an overlay prototype

## Evidence inventory (do not add tests)

| ID | Claim | Evidence | Status |
| --- | --- | --- | --- |
| P-1 | Stock Tab insertion unchanged | P-1–P-4, K-1, R-1 in `crates/pty/tests/completion_harness.rs` | confirmed |
| P-2 | Ranking is additive, not Tab order | R-2 in `tests/bash/modules.bash`: `COMPREPLY[0]` stays stock; `_MBX_COMP_ORDER` permutes | confirmed |
| P-3 | No after-every-key decoration hook | B-5 in `docs/edt-001-exact-bytes-plan.md`; ADR 0003 | confirmed |
| P-4 | Policy | No overlay. Tab stays stock. Ranked-accept chord is a later leftover. | confirmed |

If a measured result differs, keep the host bytes in that row's Status cell.
Do not “fix” stock Tab to follow ranking.

## Method

Read `docs/g3-gate-close-plan.md`, ADR 0003, ADR 0006, `bash/completion.bash`,
and `docs/comp-003-ranking-plan.md`. Confirm P-1–P-3 against existing tests.
Do not add PTY cases unless a named row has no evidence.

Product code only if a test proves default Tab now follows `_MBX_COMP_ORDER`
(it must not). Expected outcome is **no product change**.

## Docs to update (this slice)

1. `docs/roadmap.md` — `COMP-004` `not-started` → `discovery`. Immediate
   next work: do not start overlay / ghost; ranked-accept leftover later.
   Changelog row. Do not mark `COMP-004` complete.
2. `docs/architecture.md` — completion adapter: no overlay; ranking additive.
3. This file — Status `ready` → `discovery` after evidence is confirmed.
   Status column: pending → discovery plus evidence names.

## Remaining after this slice

`COMP-004` is `discovery`, not `complete`. Overlay stays unproven. A later
ranked-accept chord may use `_MBX_COMP_ORDER[0]` without changing Tab.
Ghost / highlighting stay blocked. `HIST-009` and `GIT-004` remain separate.
`HRD-001` still needs a macOS host.
