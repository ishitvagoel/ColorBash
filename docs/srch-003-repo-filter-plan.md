# SRCH-003 leftover: opt-in repository empty-line insert

Status: `complete` (2026-08-27). CLI `search repo` and writer enrichment exist
(`HIST-010`). This slice adds opt-in empty-line `\C-xh` use of
`history search repo ROOT` when `mbx repo root --cwd "$PWD"` resolves a trusted
absolute root via the ADR 0007 Git adapter. Overlay, 100k interactive
percentiles, and highlighting stay `deferred`.

## Why this slice

Interactive repo insert was marked unauthorized because Bash must not spawn Git
from repository data. The Rust helper already resolves repo root under ADR 0007;
exposing `mbx repo root` gives search a trusted root without Bash calling `git`
directly.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Opt-in repo insert + `mbx repo root` (this plan) | Closes the last Strategy A metadata filter gap. |
| — | Overlay / age columns | ADR 0003 `deferred`. |
| — | 100k interactive percentiles | `deferred` (`docs/latency-budget-deferral.md`). |
| — | Highlighting / dim paint / `COMP-004` | G5 revisit. |
| — | `HRD-001` macOS | ADR 0012 `deferred`. |

## Goal

1. `mbx repo root [--cwd PATH]` prints one sanitized absolute root line or
   nothing. Respects `MBX_DISABLE_GIT=1`. Relative cwd yields no output.
2. Empty `READLINE_LINE` + insert chord: if `MBX_SEARCH_REPO=1`, resolve root
   via `mbx repo root --cwd "$PWD"`, then query `history search repo ROOT` with
   the same bound as recent. Fall through to cwd/recent when root or matches are
   absent.
3. Default (`MBX_SEARCH_REPO` unset or not `1`) keeps cwd-then-recent.
4. Never execute inserted text. Do not spawn Git from Bash search code.

## Out of scope (hard)

- Overlay / age / status columns
- Interactive `search branch` insert
- Rebinding `\C-r`, `\C-g`, Tab, or printables
- Percentile benches
- macOS PTY matrix

## Asserts

| ID | Case | Evidence |
| --- | --- | --- |
| R-1 | `MBX_SEARCH_REPO=1` prefers a repo row over cwd/recent | Module stub |
| R-2 | `MBX_SEARCH_REPO=1` with no repo root falls through to cwd | Module stub |
| R-3 | Git worktree empty-line insert via `\C-xh` | PTY `empty_line_inserts_repo_when_opt_in` |

## Validate

```bash
cargo test -p mbx-cli repo
cargo test -p mbx-pty --test history_search empty_line_inserts_repo_when_opt_in -- --nocapture
bash tests/bash/modules.bash
bash tests/run.bash
```
