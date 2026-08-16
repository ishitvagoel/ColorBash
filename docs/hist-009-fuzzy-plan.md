# HIST-009 slice: bounded fuzzy history ranking

Status: `complete` (2026-08-16). `HIST-008` deterministic queries and the 100k
corpus exist. This packet ranks a bounded recent pool (256 rows) with exact /
prefix / substring / subsequence scores. No editor UI. Do not log command text.

## Goal

1. `mbx history search fuzzy TEXT [--limit N]` returns scored rows from the
   most recent `FUZZY_CANDIDATE_LIMIT` entries.
2. Scores: exact 300, prefix 200, substring 100, subsequence 50, else drop.
3. Ties break by `completed_at` then `event_sequence`, newest first.
4. Empty needle yields no rows. Command text is never traced (M-023).
5. `HIST-009` may move to `complete`. Percentile leftover stays `deferred`
   like other query benches unless a functional defect is proven.

## Remaining

`HIST-010` repository context still needs the `GIT-003` consumer pair.
Fuzzy is UI-free; it does not unblock ghost or Ctrl+R.
