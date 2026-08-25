# G0 leftover close: `BST-002`–`BST-004`, `PRM-001`, `PRM-009`

Status: `complete` (2026-08-25). `G0` is already `complete`. This packet
closes the remaining Phase 1–2 IDs that stayed in `validation`/`discovery`
after the gate: interactive loader contracts, MBX1 adapters, no-command-text
trace, prompt segments, and the `PRM-009` encoding reassessment. Platform
matrix leftovers stay `HRD-001`. Do not mark `SRCH-003`, `COMP-004`, or `G5`
complete.

## Why this slice

Immediate next work after `COMP-005`. These IDs have implementations and
tests; they were never given a gate-close inventory. `PRM-002` is complete,
so `PRM-009` can decide against a speculative typed-PS1 abstraction.
`SRCH-003` stays `validation` (100k leftover `deferred`). Overlay stays
`deferred`.

This is a **gate-close / decision** slice. Product code only for named rows
that lacked an assert.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `BST-002`–`BST-004`, `PRM-001`, `PRM-009` (this plan) | Named leftovers; `G0` already closed. |
| — | `SRCH-003` complete | 100k interactive leftover remains `deferred`; overlay `deferred`. |
| — | `COMP-004` overlay | Unproven continuous decoration (ADR 0003). |
| — | Broader lifecycle tracing | Already `deferred` under `BST-004`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Map each named claim to existing tests. Add an assert only when a named
   row has no evidence.
2. Record that the `HRD-001` platform matrix does **not** keep `BST-002` /
   `BST-003` in `validation` after `G0` (same leftover-owner pattern as
   overlay vs `COMP-005`).
3. Decide `PRM-009`: keep semantic composition and shared native/fallback
   contracts; do not add typed PS1 encoding or a third renderer.
4. Move `BST-002`, `BST-003`, `BST-004`, `PRM-001`, and `PRM-009` to
   `complete`. Do not start highlighting, overlay, or percentile benches.

## Out of scope (hard)

- Overlay, highlighting, dim paint
- 100k history-search interactive percentiles
- macOS / WSL pairwise matrix (`HRD-001`)
- A second prompt encoding or theme-validation framework
- Measuring or chasing `PRM-004` / adapter 5 ms leftovers
- Marking `SRCH-003`, `COMP-004`, `G5`, or `HRD-*` complete
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Logging command text (`M-023`)
- Committing unless asked

## Evidence inventory

| ID | Claim | Evidence | Gate-close status |
| --- | --- | --- | --- |
| BST-002 I-1 | Non-interactive guard | `tests/bash/smoke.bash`: `unset:unset` after sourcing `init.bash` without `-i` | satisfied |
| BST-002 I-2 | Re-source idempotence | `bash/init.bash` `_MBX_INITIALIZED`; `_mbx_install_hooks` `_MBX_HOOKS_INSTALLED`; smoke `IDEM:1:2:2` | satisfied |
| BST-002 I-3 | Status preservation | smoke `ORIGINAL_STATUS:1`; corpus marker parity | satisfied |
| BST-002 I-4 | Helper fallback | smoke missing-helper and helper-exit; PTY `missing_helper_falls_back_to_usable_prompt`, `helper_crash_degrades_without_disabling_the_shell` | satisfied |
| BST-002 I-5 | Signals / resize / `stty` | `crates/pty/tests/foundation.rs` Ctrl+C/Z, resize, `stty -g` | satisfied |
| BST-002 leftover | Linux/macOS/WSL matrix | `HRD-001` | deferred to Phase 9 — does not block this close |
| BST-003 A-1 | Coprocess / per-call adapters | `tests/bash/modules.bash` adapter contract; `tests/integration/protocol.bash` | satisfied |
| BST-003 A-2 | Helper-crash PTY | `helper_crash_degrades_without_disabling_the_shell` | satisfied |
| BST-003 leftover | Platform matrix | `HRD-001` | deferred to Phase 9 |
| BST-004 T-1 | Opt-in trace, no command text | `crates/cli/src/telemetry.rs`; `provider_failure_diagnostic_exposes_only_the_typed_kind`; `storage_failure_diagnostic_exposes_only_the_typed_kind`; write-ack digit-only samples; modules reject `MBX_DBG` | satisfied |
| BST-004 leftover | Broader lifecycle tracing | Phase 1 principal leftover | deferred — does not block this close |
| PRM-001 S-1 | Path | `injected_home_controls_path_compaction`; `long_paths_are_compacted`; `all_segment_text_crosses_the_ps1_sanitizer` | satisfied |
| PRM-001 S-2 | Git | `repository_provider_is_substitutable_and_sanitized_centrally`; provider omit/disable tests | satisfied |
| PRM-001 S-3 | Status / duration / SSH / production | `plain_prompt_preserves_segment_order`; duration boundaries; production-over-SSH native and fallback | satisfied |
| PRM-001 S-4 | Icon | `nerd_icons_replace_ascii_labels_when_requested`; ascii tests use `FLAG_ASCII_ICONS` | satisfied |
| PRM-001 S-5 | Theme | native and fallback 16/256/truecolor SGR tests | satisfied |
| PRM-009 D-1 | Reassessment | `PRM-002` complete; native + fallback already share one context/flag/safety contract (`PRM-007`). No third encoding. Keep semantic roles → theme SGR. | satisfied — no speculative abstraction |

## Method

Read `docs/g0` close notes in `docs/roadmap.md`, `docs/bash-compatibility.md`,
`docs/ux-spec.md`, `docs/architecture.md`, and `docs/solid-hardening-checklist.md`.
Confirm rows against `tests/bash/smoke.bash`, `tests/bash/modules.bash`,
`crates/pty/tests/foundation.rs`, and `crates/cli/src/prompt.rs`.

Add only:

- smoke re-source `IDEM:1:2:2` (BST-002 I-2 had a guard but no assert)
- `nerd_icons_replace_ascii_labels_when_requested` (PRM-001 icon glyphs)

Do not add overlay, macOS, or 100k cases.

## Docs to update (this slice)

1. `docs/roadmap.md` — move the five IDs to `complete`; changelog; immediate
   next work: `SRCH-003` stays `validation`; `HRD-001` remains host-blocked.
2. `docs/architecture.md` — `PRM-009` decided; no typed PS1 encoding.
3. This file — Status `ready` → `complete` after the roadmap edit lands.

## Remaining after this slice

`SRCH-003` stays `validation` (100k leftover `deferred`; overlay `deferred`).
`COMP-004` stays `discovery`. `HRD-001` / `G5` stay host-blocked. Broader
lifecycle tracing stays `deferred`. Do not start highlighting or dim paint.
