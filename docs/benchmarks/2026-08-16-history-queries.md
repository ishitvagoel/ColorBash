# History query benchmark — 2026-08-16

Environment: optimized `mbx` library tests, x86_64 WSL2/Linux 6.6.87,
Bash 5.2.21, rustc 1.97.1, 8 CPUs, load average 2.01 / 1.88 / 2.09. The run
used the uncommitted corpus/writer working tree on top of `ec1fd7c`. Seed
`0x4D425831`, 100,000 rows, 200 warm iterations after one warmup query.

Reproduce with:

```bash
cargo build --release --workspace
MBX_BENCH_ITERATIONS=200 bash scripts/benchmark-history.bash
```

```text
area=history_query_recent rows=100000 iterations=200 p50_ns=227012 p95_ns=478069 p99_ns=761996
area=history_query_prefix rows=100000 iterations=200 p50_ns=137757 p95_ns=390300 p99_ns=697857
area=history_query_prefix_common rows=100000 iterations=200 p50_ns=29232992 p95_ns=60894465 p99_ns=98728981 note=many_match_git_not_gate
area=history_query_cwd rows=100000 iterations=200 p50_ns=2033314 p95_ns=4057260 p99_ns=11623643
area=history_enqueue_microbench rows=100000 iterations=200 p50_ns=325 p95_ns=881 p99_ns=1463 note=not_prompt_boundary
```

Interpretation:

- Warm recent, selective exact-prefix, and cwd p95 are inside the `HIST-004`
  10 ms reader budget (0.48 ms, 0.39 ms, and 4.06 ms).
- A many-match `git` prefix is about 61 ms p95. Newest-first prefix search still
  sorts every match; that case is not the gate and remains a later `G2` item.
- cwd p99 was 11.6 ms on this loaded WSL host; the gate is p95.
- In-process `record()` enqueue is a microbench footnote (p95 < 1 µs here), not
  the prompt-boundary write budget.

This does not complete `G2`. Prompt-boundary write-ack percentiles are recorded
in `docs/benchmarks/2026-08-16-history-write-ack.md` (budget miss on the
development WSL host). WAL crash/corrupt recovery and WAL/SHM `0600`
never-more-permissive evidence are recorded in `crates/cli/src/storage.rs`.
The write-ack budget gate, foreign-user open, and many-match prefix latency
remain.
