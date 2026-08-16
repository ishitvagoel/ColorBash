# G4 gate-close: completion-parity gate decision

Status: `complete` (2026-08-16). `G4` gate closed. `G3` and `PRM-006` are in
`validation`. This packet closes the `G4` gate: functional parity evidence is
complete; the 5 ms adapter leftover stays `deferred`. Do not mark `COMP-001`,
`COMP-002`, or `G3` complete. Do not start `COMP-003`, popup, or a latency
bench.

## Why this slice

Immediate next work after `PRM-006`. The `G4` evidence inventory
(`docs/g4-decision-plan.md`) already maps every functional bullet to PTY tests.
The only open `G4` bullet is the provisional 5 ms adapter overhead budget.
`docs/latency-budget-deferral.md` says unmet percentiles do not block product
development — the same precedent as `G2` write-ack deferral.

This is a **gate-close decision** slice, not a new completion case and not
`COMP-003`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `G4` gate close (this plan) | Functional evidence complete; 5 ms already `deferred`. |
| 2 | `G3` continuous-decoration assessment | Unproven; needs negative-evidence spike or ADR 0003 revisit. |
| — | `COMP-003` ranking / metadata | Unblock only after `G4` closes; separate slice. |
| — | Ghost / popup / highlighting | Popup still blocked on `G3`; ghost on `G3` + async IPC. |
| — | Adapter 5 ms measurement | `deferred` with other percentiles. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Reconfirm each functional `G4` pass bullet maps to existing tests. Do not
   invent new cases.
2. Record that the deferred 5 ms leftover does **not** block `G4` gate close
   (cite `docs/latency-budget-deferral.md` and `docs/g4-decision-plan.md`).
3. Move `G4` from `validation` to `complete`. `COMP-001` and `COMP-002` stay
   `validation`.
4. Update immediate next work: do not start `COMP-003` in this slice; `G3` stays
   `validation`; continuous decoration stays unproven.

## Out of scope (hard)

- New PTY cases, fixture names, or product changes in `bash/completion.bash`
- Measuring or chasing the 5 ms adapter budget
- Weakening the 5 ms number
- Starting `COMP-003`, popup, ranking, or Git completion metadata
- Marking `COMP-001`, `COMP-002`, `G3`, `EDT-001`, or `PRM-006` complete
- Ghost, syntax highlighting, or enhanced Ctrl+R
- Rebinding printable keys or taking Readline ownership (ADR 0003)
- `set -euo pipefail` or `MBX_DBG`
- Committing unless asked
- Creating a second plan file
- Widening into `COMP-003` or popup work

## Evidence inventory (do not add tests)

Reconfirm rows from `docs/g4-decision-plan.md`. Do not rewrite case IDs or
expected GOT lines.

| G4 bullet | Evidence | Gate-close status |
| --- | --- | --- |
| `COMP_*` / `COMPREPLY` / `compopt` | H-2; P-3 nospace; P-4 default suffix | satisfied |
| Exact bytes, quoting, whitespace, suffix | P-1–P-4 | satisfied |
| Aliases, redirections, Unicode, incomplete quotes | L-1–L-4 | satisfied |
| `--` and nested `$(...)` | N-1–N-2 | satisfied |
| Unsupported / slow / stateful fallthrough | S-1–S-4 | satisfied |
| Adapter overhead <= 5 ms over stock | Not measured; `deferred` in `docs/latency-budget-deferral.md` | deferred — does not block gate close |

## Method

Read `docs/g4-decision-plan.md`, `docs/latency-budget-deferral.md`, and
`docs/roadmap.md` G4 exit criteria. Confirm functional bullets against existing
`crates/pty/tests/completion_harness.rs` and `tests/bash/modules.bash` — do
not add PTY cases unless a named row has no evidence (none expected).

Product code only if a test proves a functional `G4` bullet fails today. The
expected outcome is **no product change** — docs and gate status only.

## Docs to update (this slice)

1. `docs/roadmap.md` — `G4` status `validation` → `complete`. Phase 5 principal
   unfinished condition: `COMP-003` unblocked for planning; popup waits on
   `G3`; 5 ms leftover stays `deferred`. `COMP-002` evidence unchanged.
   Immediate next work: do not start `COMP-003` here; `G3` continuous
   decoration stays unproven. Changelog row. Do not mark `COMP-001` or
   `COMP-002` complete.
2. `docs/g4-decision-plan.md` — add a short “Gate closed” note pointing at this
   file; do not change case IDs.
3. This file — Status `ready` → `complete` after the roadmap edit lands.

## Remaining after this slice

`G4` is `complete`. `COMP-001` / `COMP-002` stay `validation`. `COMP-003` may
be planned next but is not started here. Popup stays blocked on `G3`.
Continuous decoration stays unproven (`docs/g3-decision-plan.md`). The 5 ms
leftover is reviewed before `G5`. `HRD-001` still needs a macOS host.
