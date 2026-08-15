# HIST-007 slice: 100k corpus, query percentiles, hostile inertness

Status: `complete` for the corpus, hostile-inertness, and 100k query-percentile
slice (2026-08-16). Contention, prompt-boundary write budgets, many-match prefix
latency, and permission checks beyond mode bits remain later `G2` slices.

## Goal

Produce the `HIST-004` seeded corpus and use it for two remaining `G2` evidence
items that do not require concurrent shells or a second user:

1. **Corpus.** A stable, seeded 100k-row generator matching the mix, cwd pool,
   timestamp span, and JSON Lines dump in `docs/benchmarks/history-budgets.md`.
2. **Query percentiles.** Release-mode p50/p95/p99 for warm recent, exact-prefix,
   and cwd queries against that store.
3. **Hostile inertness.** SQL fragments and terminal-control bytes in
   `command_text` / prefix / cwd stay parameterized data: the schema survives
   and queries return the original bytes.

## Out of scope

- 8-session WAL contention, writer crash, corrupt WAL/SHM, migration-on-100k
- Queue-ack measured from the Bash prompt boundary (in-process `record()` is
  only a microbench footnote, not the `G2` write budget)
- Foreign-user open, fuzzy ranking, repository context, editor UI, default-on
  capture
- Marking `G2` or `HIST-007` complete

## Method

Keep the generator in `crates/cli/src/corpus.rs` under `cfg(test)` so it is not
a runtime helper path. Unit tests cover mix, determinism, JSONL round-trip, and
hostile inertness on a small store. An ignored release test loads 100k rows
through the existing writer queue, warms the reader, and prints percentiles.
`scripts/benchmark-history.bash` runs that test and records the environment.

Raise retention for the 100k load (`MBX_HISTORY_RETENTION_ROWS` / `_DAYS`) so
seeded 2026 timestamps are not pruned. Never print command text.

## Test cases

| ID | Case | Assert |
| --- | --- | --- |
| CORP-1 | Same seed, two 200-row generations | Byte-identical `command_text`, cwd, timestamps, keys |
| CORP-2 | 10k-row mix | 55/25/10/5/3/2% ±1 count per hundred-slot mapping |
| CORP-3 | JSONL round-trip of short, long, hostile, duplicate rows | Decoder reconstructs the entry |
| CORP-4 | Hostile SQL command text stored then `recent`/`prefix`/`cwd` | Table still exists; returned text is exact |
| CORP-5 | Prefix `' OR '1'='1` and cwd `'; DROP TABLE history;--` | Not a match-all; `history` table remains |
| CORP-6 | CSI/OSC/CJK/quotes/`%`/`_` in command text | Round-trip as data; LIKE metacharacters stay literal |
| Q-1 | Ignored 100k load | `count == 100000` after shutdown drain |
| Q-2 | Warm `recent(50)`, selective `exact_prefix`, `by_cwd("/corpus/d000", 50)` | p95 < 10 ms on the recorded release run. A many-match `git` prefix is informational, not the gate. |

## Edge cases

- Do not collect 100k `HistoryEntry` values; generate by index and enqueue.
- `record()` drops the entry on `QueueFull`; retry with a clone and a short yield.
- Default retention is 200k/90d; shutdown prune would delete old seeded dates
  unless the bench raises retention.
- Row-cap prune must not run `DELETE ... OFFSET max_rows` when under the cap
  (`M-029`); the writer must commit insert batches (`M-030`). Otherwise a 100k
  load never finishes.
- Command text must not contain NUL; hostile rows use CSI/OSC/SQL/quotes only.
- In-process enqueue latency is not the prompt-side write budget.

## Implementation checklist

1. Add the seeded generator, JSONL helpers, and CORP/hostile tests.
2. Add the ignored 100k percentile test and `scripts/benchmark-history.bash`.
3. Run focused lib tests, then the release ignored bench, then `bash tests/run.bash`.
4. Store the environment and p50/p95/p99 in `docs/benchmarks/`.
5. Update `HIST-007` evidence notes only. Do not mark `G2` complete.

## Follow-on `G2` slices

Contention cases 1–6 and 8, prompt-boundary write-ack PTY, many-match prefix
latency (`git` on this corpus is ~61 ms p95), and permission checks beyond mode
bits (`docs/benchmarks/history-budgets.md`).
