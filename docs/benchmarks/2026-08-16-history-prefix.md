# History many-match prefix benchmark — 2026-08-16

Environment: optimized `mbx` library tests, x86_64 WSL2/Linux 6.6.87.2,
Bash 5.2.21, rustc 1.97.1, 8 CPUs, load average 1.25 / 0.62 / 0.24. Seed
`0x4D425831`, 100,000 rows, 200 warm iterations after one warmup query. Schema
v2 with `history_prefix_completed` covering index; `exact_prefix` uses
`INDEXED BY history_prefix_completed` and `command_text COLLATE NOCASE LIKE ?1`.

Reproduce with:

```bash
cargo build --release --workspace
MBX_BENCH_ITERATIONS=200 bash scripts/benchmark-history.bash
```

```text
area=history_query_recent rows=100000 iterations=200 p50_ns=185821 p95_ns=495270 p99_ns=596306
area=history_query_prefix rows=100000 iterations=200 p50_ns=96738 p95_ns=150466 p99_ns=194136
area=history_query_prefix_common rows=100000 iterations=200 p50_ns=2609675 p95_ns=4479265 p99_ns=6539293 note=many_match_git_not_gate
area=history_query_cwd rows=100000 iterations=200 p50_ns=1048354 p95_ns=1793341 p99_ns=2119168
area=history_enqueue_microbench rows=100000 iterations=200 p50_ns=211 p95_ns=595 p99_ns=744 note=not_prompt_boundary
```

Interpretation:

- Many-match `git` exact-prefix p95 is ~4.48 ms on this host, inside the
  `HIST-004` 10 ms reader budget (was ~61 ms p95 before schema v2; see
  `docs/benchmarks/2026-08-16-history-queries.md`).
- Selective-prefix, recent, and cwd p95 remain inside the 10 ms budget.
- Storage tests Q-A–Q-C and ADR 0008 record the covering-index migration and
  query-plan contract.

This does not complete `G2`. Foreign-user open and the prompt-boundary
write-ack percentile budget remain.
