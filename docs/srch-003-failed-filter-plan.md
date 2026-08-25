# SRCH-003 leftover: opt-in failed-command insert + Strategy A close

Status: `complete` (2026-08-25). Cwd empty-line/prefix filters and
signal/terminal-state PTY are recorded. CLI `search failed` / `search repo` /
`search branch` are on main (`HIST-010`). This packet adds opt-in empty-line
`\C-xh` use of `history search failed` and closes `SRCH-003` Strategy A
metadata filters. Overlay/age columns stay `deferred`. 100k interactive
percentiles stay `deferred`. Interactive repo-root insert stays unauthorized
(needs a trusted root; CLI `search repo` remains). Do not start highlighting
or dim paint.

## Why this slice

Immediate next work after cwd/signal. `SRCH-003` stayed `validation` because
100k and overlay were written as if they blocked the Strategy A exit. That is
the same leftover-owner mistake as overlay vs `COMP-005`. Cwd and
signal/terminal-state already have PTY. The remaining Strategy A metadata
filter that needs no overlay is status: `history search failed` on empty
`\C-xh` when `MBX_SEARCH_FAILED=1`. Default empty-line behavior stays cwd then
recent.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Opt-in failed insert + `SRCH-003` close (this plan) | CLI exists; no overlay. |
| — | Age/cwd/status columns | Overlay `deferred` (ADR 0003 / ADR 0009). |
| — | Interactive `search repo` | Needs a trusted repo root in Bash; CLI remains. |
| — | 100k interactive percentiles | `deferred` (`docs/latency-budget-deferral.md`). |
| — | Highlighting / dim paint / `COMP-004` overlay | G5 revisit. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Empty `READLINE_LINE` + insert chord: if `MBX_SEARCH_FAILED=1`, query
   `history search failed` with the same bound as recent. If that returns at
   least one line, that is the snapshot. Otherwise fall through to cwd/recent.
2. Default (`MBX_SEARCH_FAILED` unset or not `1`) keeps cwd-then-recent.
3. Typed prefix/fuzzy is unchanged (CLI `failed` takes no TEXT).
4. Never execute inserted text. Do not steal `\C-r`. Do not log command text.
5. Move `SRCH-003` to `complete` for Strategy A filters + signal/terminal-state.
   Overlay leftover stays `deferred`. 100k leftover stays `deferred`. Do not
   mark `COMP-004` or `G5` complete.

## Out of scope (hard)

- Overlay / age / status columns
- Combining failed with prefix/cwd SQL
- Interactive repo-root lookup / spawning Git from search
- Rebinding `\C-r`, `\C-g`, Tab, or printables
- Percentile benches
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Marking `COMP-004` or `HRD-*` complete

## Asserts

| ID | Case | Evidence |
| --- | --- | --- |
| F-1 | Empty chord with `MBX_SEARCH_FAILED=1` prefers a failed row over a newer success | Module stub + PTY |
| F-2 | `MBX_SEARCH_FAILED=1` with no failed rows falls through to cwd/recent | Module stub + PTY |
| F-3 | Default empty-line still prefers cwd/recent (existing C-1 / S-2) | Do not regress |

## Docs

`docs/roadmap.md`, `docs/architecture.md`, `README.md`,
`docs/latency-budget-deferral.md`, this file. Update cwd/signal plans'
remaining line to point here. Phase 8 Strategy A may move to `complete`.

## Validate

```bash
cargo test -p mbx-pty --test history_search -- --nocapture
bash tests/bash/modules.bash
bash tests/run.bash
```

## Remaining after this slice

Overlay/age columns `deferred`. 100k interactive `deferred`. Interactive repo
insert unauthorized. `HRD-001` host-blocked. Do not start highlighting.
