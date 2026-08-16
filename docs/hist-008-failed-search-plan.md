# HIST-008 leftover: failed-status history search

Status: `ready` (2026-08-16). `HIST-008` recent/prefix/cwd and `HIST-009` fuzzy
are complete. Status is already stored on every sidecar row. This packet adds a
bounded CLI filter for nonzero exit status. No editor UI. Do not log command
text. Do not bump the schema (avoids colliding with `HIST-010` v3).

## Why this slice

`HIST-010` / `GIT-003` is already in flight on a separate branch. Overlay,
ghost, highlighting, and Ctrl+R stay blocked. The remaining UI-free leftover
that can produce evidence on this host without repository fields is a failed-
command query over the existing `status` column (`docs/roadmap.md` Phase 8:
filters when indexed fields are reliable; status is recorded today).

This is a **small product slice**, same size as `HIST-009`. Not a schema
migration and not `SRCH-003`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Failed-status CLI search (this plan) | Status exists; HIST-008 query pattern exists; no schema bump. |
| — | `HIST-010` / `GIT-003` | Separate PR; do not duplicate or bump to schema v3 here. |
| — | `search branch` / repo filters | Need `HIST-010` columns. |
| — | Overlay / Ctrl+R UI | Unproven continuous decoration. |
| — | Status covering index | Optional; `history_completed` scan is enough for this CLI leftover. Schema bump would collide with `HIST-010`. |
| — | Marking `SRCH-003` complete | Interactive search UI still blocked. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. `mbx history search failed [--limit N]` returns rows with `status != 0`,
   newest first (`completed_at DESC`, then `event_sequence DESC`), capped by
   the same `--limit` / `MAX_QUERY_LIMIT` rules as other search kinds.
2. Status `0` rows are excluded. Empty result is success (no rows), not an
   error.
3. CLI stdout remains command text only (same `print_entries` path). Never
   trace command text (M-023).
4. `HistorySearch::failed` is the port method. SQLite adapter implements it.
   No MBX2 change.
5. `HIST-008` stays `complete`. `SRCH-003` stays `blocked`. `HIST-010` stays
   the next leftover.

## Out of scope (hard)

- Schema version bump or new indexes
- Repository/branch filters
- Ctrl+R / result view / insertion UI
- Logging command text
- Changing MBX1/MBX2 framing
- Percentile benches
- Marking `SRCH-003`, `HIST-010`, or `COMP-004` complete
- Overlay, ghost, highlighting

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| F-1 | Nonzero status rows returned newest first | Three rows: fail@t1, success@t2, fail@t3. `failed(10)` is `[t3, t1]` command text. | planned |
| F-2 | Status 0 excluded; limit honored | Four failed rows + successes; `failed(2)` length 2, newest two fails. | planned |
| F-3 | CLI parse | `history search failed --limit 3` → `SearchFailed { limit: 3 }`. Extra TEXT is an error. | planned |

## Docs to update

1. `docs/roadmap.md` — changelog; HIST-008 evidence note; do not mark SRCH-003 complete.
2. `docs/architecture.md` — mention `search failed`.
3. `README.md` — tryable failed search.
4. This file — `ready` → `complete` after evidence.

## Validate

```bash
cargo test -p mbx --lib failed -- --nocapture
cargo test -p mbx --lib cli::tests -- --nocapture
bash tests/run.bash
```

## Remaining after this slice

`HIST-010` / `GIT-003` repository context. Overlay/ghost/Ctrl+R stay blocked.
`SRCH-003` stays blocked.
