# PRM-009 reassessment

Status: `complete` (2026-08-16).

Close `PRM-009` (`discovery` → `complete`). `PRM-002` capability, width, and
wrap evidence is recorded. Keep the current semantic composition. Do not add
typed PS1 encoding or a second native renderer.

## Why this slice

`HIST-010`, ranked-cycle, `PRM-001`, and `PRM-006` already have open PRs.
Overlay, ghost, and highlighting stay blocked. `PRM-009` was waiting on
`PRM-002` or a second renderer. `PRM-002` is `complete` on `main`, so the
wait is over. The hardening checklist already forbids a speculative extra
abstraction.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `PRM-009` reassessment (this plan) | Named leftover; `PRM-002` wait is satisfied. |
| 2 | `HIST-010` / `GIT-003` | Separate open PRs; do not duplicate. |
| — | Typed PS1 encoding / second native renderer | No new consumer; would be a speculative abstraction. |
| — | Overlay / ghost / highlighting | Unproven continuous decoration (ADR 0003 / B-5). |
| — | `COMP-004` popup | Stays `discovery`. |
| — | `HRD-001` / `G5` | Needs a macOS host. |

## Decision

1. Native rendering stays `PromptSegmentProvider` segments plus a crate-internal
   `Theme`, with central sanitization before SGR (`crates/cli/src/prompt.rs`).
2. The process-free Bash fallback remains a parallel adapter with the same
   semantic roles and SGR tables (`PRM-007` / `PRM-002`). It is not a typed
   PS1 decoder.
3. The MBX1 `PROMPT` payload stays one sanitized prompt string. Do not encode
   role/style records on the wire.
4. Do not add a third renderer. A future second native renderer would be a
   new change axis and a new ADR.

## Goal

1. Inventory existing composition and theme evidence. Do not invent overlay
   cases or a new encoding.
2. Lock the default 256-color `Theme` table and the 16/truecolor `role_sgr`
   mappings already documented in `docs/prm-002-color-capability-plan.md`.
3. Move `PRM-009` to `complete`. Do not mark `COMP-004`, `HIST-010`,
   `PRM-001`, or `PRM-006` complete on this `main`-based branch.

## Out of scope (hard)

- A second native renderer or typed PS1 encoding
- Overlay, ghost, highlighting, enhanced Ctrl+R
- Changing MBX1 framing
- Marking `COMP-004`, `HIST-010`, `PRM-001`, or `PRM-006` complete
- `set -euo pipefail` or `MBX_DBG` in sourced Bash
- Committing unless asked

## Evidence inventory

| ID | Claim | Evidence | Gate-close status |
| --- | --- | --- | --- |
| C-1 | Native composition is ordered semantic segments | `PromptRenderer` provider list; `plain_prompt_preserves_segment_order` | satisfied |
| C-2 | Theme is a crate-internal substitutable value | `Theme` fields; `seam_contract_tests` (M-012) | satisfied |
| C-3 | Fallback shares roles/SGR, not a typed PS1 | `bash/fallback.bash` `_mbx_role_sgr`; color-depth tests in `tests/bash/modules.bash` | satisfied |
| C-4 | Wire payload is one prompt string | MBX1 `PROMPT` field; `PRM-007` hostile corpus | satisfied |
| T-1 | Default 256-color theme table is locked | `default_theme_locks_256_sgr_table` | this close |
| T-2 | 16-color and truecolor role SGR match the PRM-002 table | `role_sgr_16_and_truecolor_match_locked_tables` | this close |

## Docs

- `docs/roadmap.md`: `PRM-009` → `complete`; changelog; immediate next stays
  `HIST-010` / overlay blocked / `COMP-004` discovery.
- `docs/architecture.md`: composition decision; no typed PS1.
- `docs/solid-hardening-checklist.md`: deferred `PRM-009` wait is resolved.
- `docs/prm-006-duration-plan.md`: remaining `PRM-009` line.
