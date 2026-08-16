# ADR 0008: History many-match exact-prefix covering index (schema v2)

Status: Accepted (2026-08-16, `HIST-007` G2 evidence slice)

## Context

Exact-prefix search uses parameterized `LIKE 'prefix%'` with `COLLATE NOCASE`,
newest-first ordering, and a bounded `LIMIT`. Schema v1 indexed
`(command_text COLLATE NOCASE)` only. On a warm 100k-row corpus, selective
prefixes already meet the `HIST-004` 10 ms p95 reader budget, but a
many-match `git` prefix still materializes and sorts every matching row (~61 ms
p95 on the development WSL host; see
`docs/benchmarks/2026-08-16-history-queries.md`).

The query contract in `docs/history-phase3a-contract.md` must stay unchanged:
parameterized `LIKE`, literal `%`/`_` handling, bounded limits, and
newest-first ranking.

## Decision

Add a forward-only schema v2 migration that creates a covering prefix index and
bumps `PRAGMA user_version` to `2`:

```sql
CREATE INDEX IF NOT EXISTS history_prefix_completed
    ON history (command_text COLLATE NOCASE, completed_at DESC, event_sequence DESC);
```

Rules:

- Keep the v1 `history_prefix` index; do not drop it in this change.
- Empty stores create v1 objects plus the v2 index and land on `user_version = 2`.
- Existing v1 stores migrate on the next writer open: create the covering index,
  then set `user_version = 2`.
- Migrations remain forward-only inside `BEGIN IMMEDIATE` with version re-check
  and `ROLLBACK` on failure (`M-032`).
- Do not add FTS, trigrams, extra columns, or rewrite stored command text.
- Do not change the `exact_prefix` SQL unless a focused `EXPLAIN QUERY PLAN`
  test proves SQLite cannot use the covering index with the existing statement.
  When the planner still prefers `history_prefix`, pin the covering index with
  `INDEXED BY history_prefix_completed` and match the index collation with
  `command_text COLLATE NOCASE LIKE ?1`.

## Alternatives

- Lowering the documented 10 ms p95 budget would hide the performance defect
  without fixing query cost.
- A smaller `LIMIT` would change the query contract.
- Full-text search or fuzzy ranking belong to later roadmap items (`HIST-009`),
  not this gate.

## Consequences

- Existing v1 stores upgrade transparently on next open; readers fail closed
  until the writer completes migration (`HIST-006`).
- Disk footprint grows by one additional index; write cost on insert/prune may
  increase slightly.
- Selective-prefix and cwd query plans are unchanged.
- SQLite may still sort prefix matches because `command_text` is the leftmost
  index column; `INDEXED BY` plus covering columns still meet the 10 ms p95
  budget by avoiding table lookups on the many-match path.
- Many-match prefix latency must be re-measured on the 100k corpus after
  migration; if p95 is still ≥ 10 ms, record the miss without weakening the
  budget.

## Validation

- Storage tests Q-A–Q-C in `crates/cli/src/storage.rs`: v1→v2 migration,
  empty-store v2 landing, covering-index `EXPLAIN QUERY PLAN`, and newest-first
  ordering.
- 100k-row v1→v2 migration (M-2 in `crates/cli/src/corpus.rs`) via
  `scripts/benchmark-history-migrate.bash` recorded in
  `docs/benchmarks/2026-08-16-history-migrate.md`.
- Release benchmark via `scripts/benchmark-history.bash` recorded in
  `docs/benchmarks/2026-08-16-history-prefix.md` (or the next dated file if
  that name already exists).
