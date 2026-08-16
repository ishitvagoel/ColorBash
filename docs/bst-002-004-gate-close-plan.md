# BST-002 / BST-003 / BST-004 gate close

Status: `complete` (2026-08-16).

Close remaining Phase 1 `validation` rows now that `G0` is `complete`.
Linux/WSL foundation evidence already exists. The macOS pairwise matrix stays
`HRD-001` / `G5`. Broader lifecycle diagnostics stay deferred. This is not a
new bootstrap feature.

## Why this slice

Named leftovers that can still produce evidence on this host without
duplicating open PRs:

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `BST-002` / `BST-003` / `BST-004` gate close (this plan) | `G0` already complete; these rows still say `validation` only for `HRD-001` / deferred tracing. |
| 2 | `HIST-010` / `GIT-003` | Separate open PRs; do not duplicate. |
| — | Overlay / ghost / highlighting | Unproven continuous decoration (ADR 0003 / B-5). |
| — | `COMP-004` popup | Stays `discovery`. |
| — | `HRD-001` / `G5` | Needs a macOS host. |

## Goal

1. Inventory existing interactive-guard, adapter, and no-command-text trace
   evidence. Do not invent overlay or macOS cases.
2. Add the missing interactive re-source idempotence assert and a default-off
   helper-trace assert.
3. Move `BST-002`, `BST-003`, and `BST-004` to `complete`. Platform matrix
   remains `HRD-001`. Broader lifecycle tracing remains deferred. Do not mark
   `COMP-004`, `HIST-010`, `HRD-001`, `PRM-001`, `PRM-006`, or `PRM-009`
   complete on this `main`-based branch.

## Out of scope (hard)

- macOS / `HRD-001` PTY matrix
- Expanding `MBX_LOG` into a lifecycle diagnostic framework
- Overlay, ghost, highlighting, enhanced Ctrl+R
- Logging command text (`M-023`)
- Marking `COMP-004` or `HIST-010` complete
- `set -euo pipefail` or `MBX_DBG` in sourced Bash
- Committing unless asked

## Evidence inventory

| ID | Claim | Evidence | Gate-close status |
| --- | --- | --- | --- |
| B2-1 | Noninteractive source is a no-op | `tests/bash/smoke.bash` `_MBX_INITIALIZED-unset` | satisfied |
| B2-2 | Existing `PROMPT_COMMAND` keeps status | `tests/bash/smoke.bash` `ORIGINAL_STATUS:1` | satisfied |
| B2-3 | Corpus semantics unchanged | `tests/bash/corpus.bash` marker cmp | satisfied |
| B2-4 | Helper missing / crash degrades | smoke fallback; PTY `helper_crash_degrades_without_disabling_the_shell`; `missing_helper_falls_back_to_usable_prompt` | satisfied |
| B2-5 | Re-sourcing `init.bash` is idempotent | `tests/bash/smoke.bash` `idempotent:ok` | this close |
| B3-1 | Coprocess / per-call / fallback share context | `tests/bash/modules.bash`; `PRM-007` / `PRM-008` | satisfied |
| B3-2 | Helper crash / exit degrades | PTY foundation; smoke `RECOVERY:alive:engine=0` | satisfied |
| B3-3 | macOS adapter matrix | `HRD-001` | remains `HRD-001` — does not block this close |
| B4-1 | Provider/history diagnostics omit command text | `provider_failure_diagnostic_exposes_only_the_typed_kind`; storage diagnostic tests; M-023 | satisfied |
| B4-2 | Default install emits no helper traces | `tests/bash/smoke.bash` no `mbx trace` on stderr | this close |
| B4-3 | Broader lifecycle diagnostics | deferred known debt | deferred — does not block this close |

## Docs

- `docs/roadmap.md`: `BST-002` / `BST-003` / `BST-004` → `complete`; changelog;
  immediate next stays `HIST-010` / overlay blocked / `COMP-004` discovery.
- `docs/architecture.md`: bootstrap rows complete; tracing remains minimal.
- `docs/bash-compatibility.md`: re-source idempotence.
