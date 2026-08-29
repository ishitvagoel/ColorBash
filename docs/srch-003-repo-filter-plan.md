# SRCH-003 leftover: opt-in repository-scoped insert

Status: `complete` (2026-08-29). Closes the last authorized Strategy A
metadata-filter gap left open by `docs/srch-003-failed-filter-plan.md`
("interactive repo-root insert stays unauthorized"): interactive empty-line
`\C-xh` can now prefer repository-scoped history rows when
`MBX_SEARCH_REPO=1`. Authorization was blocked on Bash having no trusted way
to learn the current Git worktree root; `mbx repo root` closes that gap by
exposing the same ADR 0007 adapter the history writer already uses for
`repo_root`/`repo_branch` enrichment, so Bash still never calls `git` itself.

## Why this slice

`HIST-010`/`GIT-003` already gave the CLI `mbx history search repo ROOT` and
`mbx history search branch NAME`, but no interactive chord could reach them:
Bash had no sanitized way to resolve the current worktree's root without
either calling `git` directly (violating the trust boundary in ADR 0007) or
guessing from `$PWD`. `mbx repo root [--cwd PATH]` removes that blocker: it is
a thin CLI wrapper around `GitRepositoryStatusProvider::context`, the same
adapter already trusted for prompt rendering and history enrichment.

## Goal

1. `mbx repo root [--cwd PATH]`: resolves `cwd` (explicit, else the process's
   own working directory), returns the sanitized absolute Git toplevel on
   stdout, or a nonzero exit with no output when not inside a worktree, Git
   discovery is disabled (`MBX_DISABLE_GIT=1`), or discovery fails.
2. `bash/search.bash` empty-line search: when `MBX_SEARCH_REPO=1`, resolve
   the root via `mbx repo root --cwd "$PWD"`, then query
   `history search repo ROOT`. Checked after `MBX_SEARCH_FAILED` (if also
   set) and before the default cwd/recent fallback. Falls through to
   cwd/recent when the root cannot be resolved or the repo query returns no
   rows — the same fail-open shape every other opt-in empty-line filter uses.
3. Never execute inserted text. Never spawn `git` from Bash directly. Never
   log command text.
4. Default (`MBX_SEARCH_REPO` unset or not `1`) is unchanged: cwd then recent.

## Out of scope (hard)

- Upstream/branches/remotes/tags (`GIT-003` leaves these unauthorized)
- Overlay / age / status columns (still `deferred`, ADR 0003/0009)
- Combining repo with prefix/fuzzy SQL
- Rebinding `\C-r`, `\C-g`, Tab, or printables
- Percentile benches
- `set -euo pipefail` or `MBX_DBG` in sourced modules

## Asserts

| ID | Case | Evidence |
| --- | --- | --- |
| R-1 | Empty chord with `MBX_SEARCH_REPO=1` prefers a row recorded elsewhere in the same repository over a newer row recorded outside it | Module stub (`tests/bash/modules.bash`) + PTY `empty_line_inserts_repo_when_opt_in` (`crates/pty/tests/history_search.rs`), using a real `git init` worktree |
| R-2 | `MBX_SEARCH_REPO=1` outside any worktree (or on a helper failure) falls through to cwd/recent rather than failing the lookup closed | Module stub + PTY `empty_line_repo_falls_back_when_not_in_a_repository` |
| R-3 | `mbx repo root` CLI parsing (bare, `--cwd PATH`, missing value, unknown option/subcommand) | `crates/cli/src/cli.rs` unit tests |

## Docs

`docs/roadmap.md`, `docs/adr/0009-explicit-history-search-bind-x.md`
(decision 4), `README.md`, this file.

## Validate

```bash
cargo test -p mbx --lib cli::
cargo test -p mbx-pty --test history_search -- --nocapture
bash tests/bash/modules.bash
bash tests/run.bash
```

## Remaining after this slice

Overlay/age columns `deferred`. 100k interactive percentiles `deferred`.
Upstream/branches/remotes/tags stay unauthorized (`GIT-003`). `HRD-001`
macOS host-blocked (ADR 0012). Do not start highlighting or dim paint.
