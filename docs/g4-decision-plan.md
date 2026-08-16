# G4 decision: completion-parity evidence inventory

Status: `validation` (2026-08-16). COMP-001 / COMP-002 functional slices are in
`validation`. This packet records which G4 bullets have PTY evidence and
defers the 5 ms adapter leftover. Do not mark `G4`, `COMP-001`, or
`COMP-002` complete.

Do not start `COMP-003`, popup, or a latency bench.

## Why this slice

Immediate next work after S-1–S-4. Every named G4 insertion and fallthrough
context now has host PTY bytes. The remaining G4 bullet is the provisional
5 ms adapter overhead budget. Timing policy (`docs/latency-budget-deferral.md`)
says unmet percentiles do not block product development.

This is a **decision / inventory** slice, not a new completion case.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | G4 evidence inventory (this plan) | Gate is still `discovery` after functional evidence landed. |
| 2 | Adapter 5 ms budget | `deferred` with other percentiles. Do not bench here. |
| — | `COMP-003` ranking / popup | Blocked until `G4` passes. |
| — | `G3` continuous decoration | Unproven; would require a new redraw experiment or ADR 0003 revisit. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Map each G4 pass bullet to existing tests. Do not invent new cases.
2. Record the 5 ms adapter leftover in `docs/latency-budget-deferral.md`.
   Do not weaken the 5 ms number. Do not measure it here.
3. Move `G4` from `discovery` to `validation`. `COMP-001` and `COMP-002`
   stay `validation`.
4. Do not mark `G4` complete. Do not start `COMP-003`.

## Out of scope (hard)

- New PTY cases, new fixture names, or product changes in
  `bash/completion.bash`
- Measuring or chasing the 5 ms adapter budget
- Weakening the 5 ms number
- `COMP-003` ranking, descriptions, Git candidates, or popup
- Marking `G4`, `G3`, `EDT-001`, `COMP-001`, or `COMP-002` complete
- Rebinding printable keys or taking Readline ownership
- `set -euo pipefail` or `MBX_DBG`
- Committing unless asked
- Creating a second plan file

## Evidence inventory (do not add tests)

| G4 bullet | Evidence | Status |
| --- | --- | --- |
| `COMP_*` / `COMPREPLY` / `compopt` | H-2 `probe_snapshot_captures_comp_state`; P-3 nospace; P-4 default suffix | recorded |
| Exact bytes, quoting, whitespace, suffix | P-1 unique file; P-2 spaced name; P-3 / P-4 flag suffix | recorded |
| Aliases, redirections, Unicode, incomplete quotes | L-1–L-4 in `docs/comp-002-leftover-matrix-plan.md` | recorded |
| `--` and nested `$(...)` | N-1–N-2 in `docs/comp-002-dash-nested-plan.md` | recorded |
| Unsupported / slow / stateful fallthrough | S-1–S-4 in `docs/comp-002-fallthrough-plan.md` | recorded |
| Adapter overhead <= 5 ms over stock | Not measured. Record as `deferred` leftover. | deferred |

Do not rewrite those case IDs or expected GOT lines.

## Docs to update (this slice)

1. `docs/latency-budget-deferral.md` — add a row:
   `COMP-002` adapter overhead 5 ms / `deferred` / functional S-1–S-4 landed;
   no p50/p95/p99 record.
2. `docs/roadmap.md` — `G4` status `discovery` → `validation`. Phase 5
   principal unfinished condition: deferred 5 ms leftover + gate close.
   `COMP-002` evidence: S-1–S-4 landed; 5 ms deferred; `G4` in `validation`.
   Immediate next work: do not start popup; `COMP-003` stays `blocked`;
   `G3` stays `validation`. Changelog row.
3. This file — Status `ready` → `validation` after the doc edits land.

## Remaining after this slice

`G4` is `validation`, not `complete`. `COMP-003` stays `blocked`. Popup
stays blocked on `G3` and `G4`. Continuous decoration stays unproven
(`docs/edt-001-exact-bytes-plan.md` B-5). `HRD-001` still needs a macOS
host. The 5 ms leftover is reviewed before `G5`, not as Immediate next
work.
