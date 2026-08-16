# PRM-006 slice: duration-timing policy decision

Status: `ready` (2026-08-16). `G3` and `G4` are in `validation`. This packet
records the duration-timing decision: remain opt-in; do not compose an
unknown `DEBUG` trap; do not add a bash-preexec adapter. Do not mark
`PRM-006`, `G3`, or `G4` complete.

Do not start ghost, popup, highlighting, or `COMP-003`.

## Why this slice

Immediate next work after the `G3` inventory. Architecture reassessment
item 6 and `PRM-006` are still `discovery`. Existing smoke already proves
default install leaves `DEBUG` alone and opt-in records duration. This is
a **decision / inventory** slice, same size as `docs/g3-decision-plan.md`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Duration policy decision (this plan) | Named leftover; host already has smoke evidence. |
| 2 | bash-preexec / DEBUG composition adapter | Unsafe without a proven trap-inspection primitive. |
| — | Ghost / popup / `COMP-003` | Blocked on `G3` / `G4` complete. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Map each duration policy claim to existing tests. Do not invent extra
   frameworks or a second hook.
2. Decide: duration stays opt-in via `MBX_ENABLE_DURATION_TIMING=1`. Default
   install never installs a `DEBUG` trap. Do not compose or evaluate an
   unknown `DEBUG` trap.
3. Do not add a bash-preexec adapter in this slice.
4. Move `PRM-006` from `discovery` to `validation`. Keep `G3` / `G4` in
   `validation`. Do not mark `PRM-006` complete.

## Out of scope (hard)

- Composing, wrapping, or `eval`-ing an existing `DEBUG` trap
- Installing a bash-preexec / precmd framework
- Changing default-off timing
- Ghost, popup, highlighting, `COMP-003`
- Percentile benches
- Marking `PRM-006`, `G3`, `G4`, `EDT-001`, `COMP-001`, or `COMP-002` complete
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Committing unless asked
- Creating a second plan file
- Widening this packet

## Evidence inventory (do not add cases unless a named row is missing)

| ID | Claim | Evidence | Status |
| --- | --- | --- | --- |
| D-1 | Default install does not install `DEBUG` | `tests/bash/smoke.bash`: after `source init.bash`, `trap -p DEBUG` matches the pre-source value; `_MBX_DURATION_TIMING=0` | pending |
| D-2 | Existing `DEBUG` trap is preserved when timing is off | Same smoke case: `preserved:0` | pending |
| D-3 | Opt-in records duration | `MBX_ENABLE_DURATION_TIMING=1`; after `sleep 0.02`, `_MBX_LAST_DURATION_MS` is a number `>= 10` | pending |
| D-4 | Policy | Remain opt-in. Do not compose unknown `DEBUG`. Do not add a preexec adapter. History and ranking keep nullable duration. | pending |

If a measured result differs, keep the host bytes and write them into that
row's Status cell. Do not “fix” trap composition.

## Method

Read `bash/hooks.bash`, `docs/research/bash-readline-investigation.md`
(Before execution), and `docs/bash-compatibility.md` hook findings. Confirm
D-1–D-3 against `tests/bash/smoke.bash`. Do not add PTY cases unless a
named row has no evidence.

Product code in `bash/hooks.bash` only if a test proves the **default**
install replaces `DEBUG` today. The expected outcome is **no product
change** beyond this plan's status column and roadmap/architecture
pointers.

## Docs to update (this slice)

1. `docs/roadmap.md` — `PRM-006` `discovery` → `validation`. Immediate
   next work: do not start ghost / popup / `COMP-003`. Changelog row.
2. `docs/architecture.md` reassessment item 6 — point at this plan;
   duration remains opt-in.
3. This file — Status `ready` → `validation` after those edits. Status
   column: pending → validation plus the smoke file name.

## Remaining after this slice

`PRM-006` is `validation`, not `complete`. A later adapter would need a
proven way to inspect the caller's `DEBUG` trap from a sourced file
without changing context. Ghost / popup / `COMP-003` stay blocked.
`PRM-009` stays `discovery`. `HRD-001` still needs a macOS host.
