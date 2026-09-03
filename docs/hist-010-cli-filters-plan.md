# HIST-010 leftover: branch/failed CLI filters + PRM-006 gate close

Status: `complete` (2026-08-16). Schema v3 already stores `repo_root` /
`repo_branch`. Status is already on every row. `G3` / `G4` are `complete`.
This packet adds the remaining UI-free CLI filters and closes `PRM-006`.
Do not log command text. Do not bump the schema. Do not mark overlay /
`COMP-004` / `SRCH-003` complete.

## Why this slice

`HIST-010` already records root/branch and `search repo`. Named leftovers
that can produce evidence without overlay:

1. `mbx history search branch NAME` — exact `repo_branch` match.
2. `mbx history search failed` — `status != 0` (same leftover as the
   main-line failed-search PR; land it here so this branch has the full
   CLI filter set).
3. `PRM-006` gate close — D-1–D-4 exist; `G3`/`G4` no longer block.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | These leftovers | Unblocked; no overlay; no schema bump. |
| — | Overlay / ghost / Ctrl+R | Unproven continuous decoration. |
| — | GIT-003 upstream/remotes/tags | Unauthorized until a later consumer. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. `HistorySearch::by_branch` / `failed`; CLI kinds `branch` and `failed`.
2. Branch query is exact `repo_branch = ?`, newest first, bounded `--limit`.
   NULL/other branches excluded. CLI search stays process-free (no Git).
3. Failed query is `status != 0`, newest first, bounded `--limit`.
4. Stdout remains command text only.
5. `PRM-006` moves to `complete`. Remain opt-in; do not compose `DEBUG`.
6. `SRCH-003` stays `blocked`. `COMP-004` stays `discovery`.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| B-1 | Exact branch, newest first | Rows on `main` and `hist-branch`; `by_branch("hist-branch")` returns only that branch, newest first. | complete — `by_branch_matches_exact_name_newest_first` |
| B-2 | CLI parse | `search branch hist-branch --limit 2`. Missing NAME errors. | complete — `crates/cli/src/cli.rs` |
| F-1 | Failed newest first; status 0 excluded; limit | `failed_returns_nonzero_status_newest_first`: nonzero-status rows newest first, `status = 0` excluded, bounded `--limit`. (This row originally pointed at a `docs/hist-008-failed-search-plan.md` from the main-line failed-search branch; that plan never landed on main, so the asserts are stated here. The interactive opt-in filter is `docs/srch-003-failed-filter-plan.md` F-1/F-2.) | complete — `failed_returns_nonzero_status_newest_first` |
| F-2 | CLI parse failed | `search failed --limit 3`; extra TEXT errors. | complete — `crates/cli/src/cli.rs` |
| D-G | PRM-006 gate | D-1–D-4 already in smoke; mark `complete`. | complete — `docs/prm-006-duration-plan.md` |

## Docs

`docs/roadmap.md`, `docs/architecture.md`, `README.md`,
`docs/prm-006-duration-plan.md`, this file.

## Validate

```bash
cargo test -p mbx --lib -- --nocapture
bash tests/run.bash
```

## Remaining

Overlay / ghost / highlighting / Ctrl+R stay blocked. `HRD-001` needs macOS.
`COMP-004` stays `discovery`. `SRCH-003` stays `blocked`.
