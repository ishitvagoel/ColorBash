# G5 gate-close: Strategy A MVP on Linux

Status: `complete` (2026-08-27). `G0`–`G4`, Phases 3–5 and 8 Strategy A,
`HRD-002`, `HRD-004`, and Linux `HRD-001` L-1–L-5 are recorded. macOS
`HRD-001` is `deferred` (ADR 0012). `HRD-003` percentiles stay `deferred`
(`docs/latency-budget-deferral.md`). Overlay, highlighting, and dim paint stay
`deferred` (ADR 0003). Do not mark `COMP-004` complete.

## Why this slice

The product owner authorized deferring macOS platform-matrix work and
completing every other roadmap requirement this Linux host can evidence.
This is a **gate-close decision** slice, not new product scope.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `G5` Strategy A close (this plan) | All non-deferred `HRD-*` items and matrix leftovers on this host are mapped to tests. |
| — | macOS `HRD-001` pairwise | `deferred` per ADR 0012; needs a macOS host. |
| — | `HRD-003` release latency | Percentiles `deferred`; do not chase product-code latency. |
| — | GUI overlay / highlighting / dim paint | ADR 0003; `COMP-004` stays `discovery`. |
| — | `GIT-005` provider SDK | Post-MVP `deferred`. |

## Goal

1. Map each named `G5` / Phase 9 matrix claim to existing tests. Add an assert
   only when a named row has no evidence.
2. Record macOS deferral (ADR 0012) and close `HRD-001` for the Linux slice.
3. Move `G5` and Phase 9 to `complete`. Keep revisit owners explicit.
4. Do not start overlay, highlighting, dim paint, or percentile benches.

## Out of scope (hard)

- macOS / fake Darwin PTY runs
- Overlay, highlighting, dim paint, or `COMP-004` complete
- `HRD-003` / `PRM-004` percentile measurement or product-code latency work
- Interactive `search repo` insert (unauthorized)
- Git upstream/remotes/tags (unauthorized)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Committing unless asked

## Evidence inventory

| Claim | Evidence | Status |
| --- | --- | --- |
| Prompt lifecycle + command execution | `crates/pty/tests/foundation.rs` `prompt_lifecycle_renders_and_runs_commands` | satisfied |
| Missing helper fallback | `foundation.rs` `missing_helper_falls_back_to_usable_prompt`; `tests/bash/smoke.bash` | satisfied |
| Helper crash recovery | `foundation.rs` `helper_crash_degrades_without_disabling_the_shell`; smoke recovery | satisfied |
| Ctrl+C at prompt | `foundation.rs` `ctrl_c_restores_a_usable_prompt`; `editor_bind_x` cancel; `history_search` cancel; M-051 engine coproc | satisfied |
| Ctrl+Z / background job | `foundation.rs` `ctrl_z_stops_a_job_and_returns_to_the_prompt`; `editor_bind_x` ctrl-z | satisfied |
| Resize + `stty -g` | `foundation.rs` `resize_updates_lines_and_columns`, `ctrl_c_and_resize_preserve_stty_settings`; `multiline_width.rs` | satisfied |
| Nested interactive Bash | `hrd001_linux.rs` L-1 | satisfied |
| SSH prompt context | `hrd001_linux.rs` L-2 | satisfied |
| Login shell | `hrd001_linux.rs` L-3 | satisfied |
| Fullscreen vim restore | `hrd001_linux.rs` L-4 | satisfied |
| tmux session | `hrd001_linux.rs` L-5 (`/usr/bin/tmux`) | satisfied |
| emacs + vi-insert editing | `editor_bind_x.rs`, `ghost.rs` vi cases | satisfied |
| Multiline / wrap / wide glyphs | `multiline_width.rs`, `ghost.rs` PS2 | satisfied |
| Hostile input / protocol / privacy | `docs/hrd-002-hostile-audit-plan.md` H-1–H-11 | satisfied |
| Install / disable / removal / crash | `docs/hrd-004-lifecycle-plan.md` L-1–L-6; smoke bashrc sentinel | satisfied |
| `PROMPT_COMMAND` ordering | `tests/bash/smoke.bash` status preservation + idempotence | satisfied |
| Bash compatibility corpus | `tests/bash/smoke.bash` + `corpus.bash` | satisfied |
| Canonical CI suite | `.github/workflows/ci.yml` → `bash tests/run.bash` | satisfied |
| macOS pairwise matrix | ADR 0012 `deferred` | deferred — G5 revisit |
| Release percentile matrix | `HRD-003` `deferred` | deferred |
| GUI overlay / highlighting | ADR 0003 `deferred`; `COMP-004` `discovery` | deferred |

## Docs to update

1. `docs/roadmap.md` — `G5` and Phase 9 `complete`; `HRD-001` Linux
   `complete`, macOS `deferred`; Immediate next work is G5 revisit only.
2. `docs/latency-budget-deferral.md` — macOS matrix row.
3. `README.md` — Strategy A MVP status; macOS deferred, not blocked.
4. `docs/hrd-001-linux-pairwise-plan.md` — parent `HRD-001` Linux slice closed.
5. This file — Status `complete` after the roadmap edit.

## Remaining (G5 revisit)

- macOS `HRD-001` pairwise PTY when a host is available (ADR 0012)
- `HRD-003` / `PRM-004` / adapter percentiles when ratified or re-benchmarked
- Overlay (`COMP-004`), highlighting (`HLT-*`), dim paint (ADR 0003)
- `GIT-005` provider SDK
