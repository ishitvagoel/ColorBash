# COMP-005 gate-close: Strategy A insert/fallthrough

Status: `complete` (2026-08-25). `G4` is `complete`. Ranked-accept A-1–A-6
and ranked-cycle C-1–C-6 are recorded. This packet closes `COMP-005`:
Strategy A insert/fallthrough uses existing G4/COMP-002 parity plus ranked
chords. GUI overlay stays `deferred`. Do not mark `COMP-004` complete.

## Why this slice

Immediate next work after ranked-cycle `\C-xn` / `\C-xp`. `COMP-005` was
listed `blocked` on a `COMP-004` overlay leftover. Overlay is not a
Strategy A insert/fallthrough blocker: Tab stays stock, ranked-accept and
ranked-cycle are explicit `bind -x`, and Git kinds are additive (`GIT-004`).
The 5 ms adapter leftover stays `deferred` (`docs/latency-budget-deferral.md`).

This is a **gate-close decision** slice, not a new completion case and not
an overlay.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `COMP-005` Strategy A close (this plan) | Functional insert/fallthrough evidence exists; overlay is not required. |
| — | GUI overlay / `COMP-004` complete | Unproven continuous decoration (ADR 0003). `COMP-004` stays `discovery`. |
| — | Adapter 5 ms measurement | `deferred` with other percentiles. |
| — | Highlighting / dim paint | `deferred`; G5 revisit. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Reconfirm each Strategy A insert/fallthrough claim maps to existing tests.
   Do not invent overlay cases.
2. Record that GUI overlay does **not** block `COMP-005` (cite
   `docs/comp-004-popup-plan.md` P-1–P-4 and ADR 0003).
3. Move `COMP-005` from `blocked` to `complete`. `COMP-004` stays
   `discovery`. Overlay stays `deferred`.
4. Do not start highlighting, dim paint, or a percentile bench.

## Out of scope (hard)

- New PTY cases, fixture names, or product changes in `bash/completion.bash`
- Drawing a menu, overlay, or candidate list
- Rebinding Tab, arrows, or printable keys (ADR 0003)
- Measuring or chasing the 5 ms adapter budget
- Marking `COMP-004` complete
- Ghost, syntax highlighting, or enhanced Ctrl+R product code
- `set -euo pipefail` or `MBX_DBG`
- Committing unless asked
- Creating a second plan file
- Widening into overlay or continuous decoration

## Evidence inventory (do not add tests)

Reconfirm rows from `docs/g4-decision-plan.md`,
`docs/comp-004-ranked-accept-plan.md`, `docs/comp-004-ranked-cycle-plan.md`,
and `docs/git-004-kinds-plan.md`. Do not rewrite case IDs or expected GOT
lines.

| COMP-005 claim | Evidence | Gate-close status |
| --- | --- | --- |
| `COMP_*` / `COMPREPLY` / `compopt` | H-2; P-3 nospace; P-4 default suffix | satisfied |
| Exact bytes, quoting, whitespace, suffix | P-1–P-4 | satisfied |
| Aliases, redirections, Unicode, incomplete quotes | L-1–L-4 | satisfied |
| `--` and nested `$(...)` | N-1–N-2 | satisfied |
| Unsupported / slow / stateful fallthrough | S-1–S-4 | satisfied |
| Ranked-accept inserts top-ranked bytes without changing Tab | A-1–A-6 | satisfied |
| Ranked-cycle next/prev without overlay | C-1–C-6 (`\C-xn` / `\C-xp`; 2026-08-25 PTY 40/40) | satisfied |
| Git candidates when enabled | `GIT-004`; `git_kinds_tab_keeps_prefix`; `git_kinds_ranked_accept_replaces_ref` | satisfied |
| GUI overlay / popup navigation | P-1–P-4; ADR 0003 B-5 | deferred — does not block Strategy A |
| Adapter overhead <= 5 ms over stock | Not measured; `deferred` in `docs/latency-budget-deferral.md` | deferred — does not block gate close |

## Method

Read `docs/g4-gate-close-plan.md`, `docs/comp-004-popup-plan.md`, ADR 0003,
ADR 0006, and `docs/roadmap.md` Phase 5 exit criteria. Confirm functional
bullets against existing `crates/pty/tests/completion_harness.rs` and
`tests/bash/modules.bash` — do not add PTY cases unless a named row has no
evidence (none expected).

Product code only if a test proves a functional insert/fallthrough bullet
fails today. The expected outcome is **no product change** — docs and gate
status only. Ranked-cycle rustfmt in `crates/pty/tests/completion_harness.rs`
is allowed so the canonical suite can run.

## Docs to update (this slice)

1. `docs/roadmap.md` — `COMP-005` `blocked` → `complete`. Phase 5 principal
   unfinished condition: `COMP-004` overlay leftover (`discovery`); overlay
   `deferred`. Immediate next work: do not start overlay; `SRCH-003` stays
   `validation`. Changelog row. Do not mark `COMP-004` complete.
2. `docs/comp-004-ranked-cycle-plan.md` — C-1–C-6 Status `planned` →
   `complete` after the 2026-08-25 PTY run.
3. `docs/architecture.md` — `COMP-005` Strategy A insert/fallthrough closed;
   overlay still not in this milestone.
4. This file — Status `ready` → `complete` after the roadmap edit lands.

## Remaining after this slice

`COMP-005` is `complete`. `COMP-004` stays `discovery` until overlay
strategy is resolved or an ADR narrows it. GUI overlay remains unproven.
`SRCH-003` Strategy A is closed in `docs/srch-003-failed-filter-plan.md`
(100k interactive leftover `deferred`).
`HRD-001` still needs a macOS host.
