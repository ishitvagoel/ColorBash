# History sidecar datasets, contention cases, and benchmark budgets

Date: 2026-08-15. Status: `HIST-004` deliverable; budgets are provisional until
ratified by the G1/G2 gates and benchmark evidence.

This document defines the reproducible workloads and acceptance budgets for the
history sidecar. It is the input to `HIST-013` (SQLite packaging decision) and
the measurement target for `HIST-009` and the `G2` search/write gates.

## Method

- Release-mode builds only (`--release`, workspace default LTO profile).
- Benchmarks run on the development WSL2/Linux environment (Bash 5.2.21) and
  must be repeated on Linux CI and macOS before G2; the environment and machine
  are recorded with every run.
- Percentiles are p50/p95/p99 over the stated iteration count; outliers are
  reported, not discarded.
- The prompt-side write budget is measured from the Bash prompt boundary
  (queue acknowledgement), not from the writer's commit.

## Datasets

### Synthetic 100k-row corpus

One canonical generator produces a stable, seeded corpus so every benchmark and
regression run uses identical rows:

| Mix | Proportion | Example shape |
| --- | ---: | --- |
| short commands | 55% | `ls`, `git status`, `cd ~/src` |
| medium commands | 25% | flags and two arguments, 20–80 bytes |
| long commands | 10% | 120–4,000 bytes, `printf` and tool pipelines |
| multiline folded | 5% | `if ...; then ...; fi` folded single entries |
| unicode/hostile | 3% | CJK paths, control characters, `%`, quotes, `$` |
| duplicates | 2% | exact repeats and near-duplicates for ranking |

- Text is deterministic (seeded PRNG), bounded to the accepted maximum
  command length, and contains no NUL (rejected before storage).
- cwd values come from a fixed 200-directory pool; timestamps span 90 days.
- The corpus is emitted as JSON Lines so both Rust and Bash consumers can load
  it without a shared binary format.

### Real-world samples

- 10k-row sample from a developer shell history for distribution shape
  validation (shape statistics only; no command text leaves the machine).
- Empty store and single-row stores for edge-case latency.

## Contention cases

Run against a populated store with the writer active:

1. 8 concurrent recorder sessions, each with its own writer connection, writing
   distinct commands through SQLite WAL.
2. Two sessions with overlapping `session_id`-independent queues writing
   concurrently.
3. Lock contention: one reader while the writer prunes.
4. Writer crash mid-commit (kill -9) and restart; retries must not duplicate.
5. Corrupt WAL/SHM file; the writer must degrade without destroying the store.
6. Full queue: producers keep producing after a slow writer; prompt-side
   acknowledgement must stay inside budget.
7. Permission tests: `0700` directory, `0600` database/WAL/SHM; a foreign user
   cannot open the store.
8. Migration from v0 (empty) through v1 and a hypothetical v2 on a 100k-row
   store.

## Budgets

| Area | Budget | Measured at |
| --- | --- | --- |
| Write queue acknowledgement | p95 < 2 ms, p99 < 5 ms | Bash prompt boundary |
| Exact-prefix query (100k rows) | p95 < 10 ms | reader, warm cache |
| Recent query (100k rows) | p95 < 10 ms | reader, warm cache |
| cwd query (100k rows) | p95 < 10 ms | reader, warm cache |
| Ranked/fuzzy query (100k rows) | p95 < 50 ms | reader, bounded candidate set |
| Prompt side, any storage failure | within one render cycle, never blocking | prompt boundary |
| Writer commit behind the queue | unmeasured end-to-end; only queue ack is budgeted | writer |

The 100k-row corpus is the required scale for `G2`; a 1M-row run is recorded as
an informational stress datapoint, not a gate. Query p95 evidence for recent,
selective prefix, and cwd is in `docs/benchmarks/2026-08-16-history-queries.md`.
A many-match `git` prefix currently misses the 10 ms p95 budget.

## Reporting

Each benchmark run records:

- environment (OS, kernel, Bash version, machine, release binary commit);
- dataset size and seed;
- iteration count;
- p50/p95/p99 table plus the recorded machine load;
- any budget that failed, with the failing scenario.

Results are stored in `docs/benchmarks/` with a `YYYY-MM-DD-topic.md` name
matching the existing convention.
