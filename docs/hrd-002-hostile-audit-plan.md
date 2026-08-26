# HRD-002: Hostile input, protocol bounds, privacy, and no-execution audit

Status: `complete` (2026-08-26). Strategy A feature exits are recorded.
This packet is the Phase 9 hostile/privacy/no-execution audit. It inventories
existing evidence, closes the search/editor C0 insert gap, and does **not**
close `G5` or `HRD-001`. Overlay, highlighting, dim paint, and percentile
benches stay `deferred`.

## Why this slice

Immediate next work after Strategy A MVP close. `HRD-001` is host-blocked on
macOS. `HRD-003` is a percentile leftover (`docs/latency-budget-deferral.md`).
`HRD-002` can produce durable Linux evidence now: protocol bounds, PS1
sanitization, opt-in history privacy, and insert-without-execution.

Search previously inserted sidecar stdout into `READLINE_LINE` without the
C0/DEL gate ghost already applies (`_mbx_ghost_usable_match`). The editor
token is caller environment and had the same gap. Those inserts are not
`eval`, but ESC in the line buffer is terminal injection when Readline
redisplays.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `HRD-002` audit + C0 insert gate (this plan) | Runnable on this Linux host. |
| 2 | `HRD-004` install/disable/removal | Next Phase 9 slice. |
| — | `HRD-001` macOS pairwise matrix | Needs a macOS host. |
| — | `HRD-003` release latency | Percentiles `deferred`; do not chase. |
| — | Overlay / highlighting / dim paint | ADR 0003; G5 revisit. |

## Goal

1. Map each named `HRD-002` claim to tests. Add an assert only when a named
   row has no evidence or a gap is proven.
2. Refuse C0/DEL in history-search matches and the editor insert token the
   same way ghost refuses a control suffix (ADR 0010).
3. Move `HRD-002` to `complete`. Do not mark `G5`, `HRD-001`, `HRD-003`,
   `HRD-004`, or `COMP-004` complete.

## Out of scope (hard)

- Overlay, highlighting, dim paint
- macOS / WSL pairwise matrix (`HRD-001`)
- Measuring or chasing `HRD-003` / `PRM-004` percentiles
- Interactive `search repo` insert (unauthorized; needs a trusted Bash root)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Logging command text (`M-023`)
- Marking `G5` complete
- Committing unless asked

## Evidence inventory

| ID | Claim | Evidence | Status |
| --- | --- | --- | --- |
| H-1 | Hostile protocol bytes round-trip; unescaped C0 fails closed | `tests/bash/modules.bash` field codec + raw-control scan; `crates/protocol` NUL/percent; MBX2 C0 in `transport.rs` | satisfied |
| H-2 | 64 KiB payload bound, terminator-independent | Rust/Bash `MAX-1`/`MAX`/`MAX+1` EOF/LF/CRLF (`BST-006`) | satisfied |
| H-3 | Native/fallback/per-call PS1 share one hostile corpus | `tests/bash/modules.bash` C0/DEL/`$`/backtick sanitizer matrix (`PRM-007`) | satisfied |
| H-4 | MBX2 ERROR never echoes untrusted kind text | M-048; `encode_mbx2_error`; `unknown_kind_is_unsupported_and_does_not_echo` | satisfied |
| H-5 | Ghost suffix refuses C0/DEL and does not execute | `_mbx_ghost_usable_match`; module escape reject; G-1 / C-1 Enter/Ctrl+C PTY | satisfied |
| H-6 | Search insert never `eval`s; C0/DEL matches are skipped | `_mbx_text_has_c0_or_del` in `_mbx_search_helper`; module stub with ESC then clean line; PTY insert-without-Enter | satisfied |
| H-7 | Editor token with C0/DEL is a no-op | `_mbx_editor_insert_token`; module contract | satisfied |
| H-8 | Sourced modules never `eval` | module scan of `bash/*.bash` | satisfied |
| H-9 | History opt-in, no command-text log, HISTFILE invariance | `MBX_HISTORY=1` only (M-024); `MBX_DBG` forbidden (M-023); PTY invariance | satisfied |
| H-10 | Hostile SQL/command text stays inert data | `hostile_sql_and_control_rows_stay_inert`; PTY `hostile_command_text_remains_inert` | satisfied |
| H-11 | Git does not execute repo-configured helpers | ADR 0007; `hostile_repository_fsmonitor_configuration_is_not_executed` | satisfied |
| leftover | tmux/SSH/fullscreen pairwise | `HRD-001` / `G5` matrix | not this slice |

## Asserts added this slice

| ID | Evidence |
| --- | --- |
| H-6a | Helper that prints `ESC` then `echo MBX_HRD:ok` inserts only the clean line |
| H-6b | Helper that prints only `ESC` leaves `READLINE_LINE` unchanged |
| H-7a | `MBX_EDITOR_INSERT_TOKEN` containing ESC leaves the line unchanged |
| H-8a | Every `bash/*.bash` file is free of `eval ` |

## Docs to update

1. `docs/roadmap.md` — `HRD-002` `not-started` → `complete`; Immediate next
   work becomes `HRD-004`; changelog. Do not mark `G5` complete.
2. `docs/architecture.md` — note the C0/DEL insert gate on search/editor.
3. `README.md` — Phase 9 hostile audit recorded; macOS matrix still waiting.
4. This file — Status `complete` after the roadmap edit.

## Remaining

`HRD-004` is next. `HRD-001` / `G5` stay host-blocked. Do not start
highlighting or dim paint.
