# PRM-006 gate close

Status: `complete` (2026-08-16).

Close `PRM-006` (`validation` → `complete`) from the accepted duration-timing
policy. Remain opt-in. Do not compose an unknown `DEBUG` trap. Do not add a
bash-preexec adapter. This is not a new timing feature.

## Why this slice

`HIST-010` / ranked-cycle / failed-search already have open PRs. Overlay,
ghost, and highlighting stay blocked. `PRM-006` is the next unique leftover
that can produce evidence on this host: D-1–D-4 already exist, and the MVP
decision is to keep duration opt-in rather than invent trap composition.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `PRM-006` gate close (this plan) | Named leftover; smoke evidence exists. |
| 2 | `HIST-010` / `GIT-003` | Separate open PRs; do not duplicate. |
| — | bash-preexec / `DEBUG` composition | Unsafe without a proven trap-inspection primitive. |
| — | Overlay / ghost / highlighting | Unproven continuous decoration (ADR 0003 / B-5). |
| — | `COMP-004` popup | Stays `discovery`. |
| — | `HRD-001` / `G5` | Needs a macOS host. |

## Goal

1. Reconfirm D-1–D-4. Add the missing default-empty `DEBUG` assert: sourcing
   `init.bash` without `MBX_ENABLE_DURATION_TIMING=1` must leave `trap -p DEBUG`
   unchanged when no trap was installed.
2. Accept remaining opt-in as the MVP policy. A later adapter would need a
   proven way to inspect the caller's `DEBUG` trap from a sourced file without
   changing context (`docs/research/bash-readline-investigation.md`).
3. Move `PRM-006` to `complete`. Do not mark `COMP-004`, overlay, or
   `HIST-010` complete. Do not compose `DEBUG`.

## Out of scope (hard)

- Composing, wrapping, or `eval`-ing an existing `DEBUG` trap
- Inspecting `DEBUG` from inside sourced modules via command substitution
- Installing a bash-preexec / precmd framework
- Changing default-off timing
- Overlay, ghost, highlighting, enhanced Ctrl+R
- Marking `COMP-004` or `HIST-010` complete
- `set -euo pipefail` or `MBX_DBG` in sourced Bash
- Committing unless asked

## Evidence inventory

| ID | Claim | Evidence | Gate-close status |
| --- | --- | --- | --- |
| D-1 | Default install does not install `DEBUG` when a trap already exists | `tests/bash/smoke.bash` `preserved:0` | satisfied |
| D-1b | Default install does not install `DEBUG` when none existed | `tests/bash/smoke.bash` `unset-debug:0` | this close |
| D-2 | Existing `DEBUG` trap is preserved when timing is off | Same `preserved:0` case | satisfied |
| D-3 | Opt-in records duration | `MBX_ENABLE_DURATION_TIMING=1`; `_MBX_LAST_DURATION_MS` is a number `>= 10` after `sleep 0.02` | satisfied |
| D-4 | Policy | Remain opt-in. Do not compose unknown `DEBUG`. Do not add a preexec adapter. History keeps nullable duration. | satisfied — this plan |

Product code in `bash/hooks.bash` only if a test proves the **default**
install replaces `DEBUG` today. Expected outcome is the D-1b smoke assert
plus docs.

## Docs

- `docs/roadmap.md`: `PRM-006` → `complete`; changelog; immediate next stays
  `HIST-010` / overlay blocked / `COMP-004` discovery.
- `docs/architecture.md` item 6: point at this close; duration remains opt-in.
- `docs/prm-006-duration-plan.md`: status `complete`.
- `docs/bash-compatibility.md`: default install does not install `DEBUG`.
