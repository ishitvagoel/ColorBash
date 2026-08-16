# History v1→v2 migration benchmark — 2026-08-16

Environment: optimized `mbx` library tests, x86_64 WSL2/Linux 6.6.87.2,
Bash 5.2.21, rustc 1.97.1, 8 CPUs, load average not recorded at run time. Seed
`0x4D425831`, 100,000 rows inserted into a raw schema-v1 WAL store, then opened
with `QueuedHistoryStore::open_with_limits` to trigger v1→v2 migration.

Reproduce with:

```bash
cargo build --release --workspace
bash scripts/benchmark-history-migrate.bash
```

```text
area=history_migrate_v1_v2 rows=100000 elapsed_ms=2437
```

Interpretation:

- The 100k-row v1 corpus migrates to schema v2 without row loss; both
  `history_prefix` and `history_prefix_completed` exist afterward; `count ==
  100000`; many-match `git` prefix remains newest-first.
- Wall time includes raw v1 corpus fill, first open (migrate), writer shutdown,
  and post-migrate assertions on a second open.
- There is no separate latency budget for migration; this records the measured
  wall time for `HIST-004` contention case 8.

This does not complete `G2`. Foreign-user open and the prompt-boundary
write-ack percentile budget remain.
