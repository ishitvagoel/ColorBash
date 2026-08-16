# History write-ack benchmark — cloud remeasure 2026-08-16

Environment: release `mbx` PTY benchmark via
`crates/pty/tests/history_write_ack.rs`, x86_64 Linux 6.12.94+ (cloud agent),
Bash 5.2.21, rustc 1.97.1, load average 0.00 / 0.06 / 0.06. Production
`MBX_HISTORY_TIMEOUT` and `MBX_IPC_TIMEOUT` (0.10 s), `MBX_DISABLE_GIT=1`,
200 admitted `echo bench-{n}` commands after first-prompt skip. Each command
waits for its echoed marker and the next `> ` before continuing (M-019-safe PTY
sync). The harness sets `MBX_TEST_BIN` to `${CARGO_TARGET_DIR:-<workspace>/target}/release/mbx`
so the helper matches the `cargo build --release` just performed.

Reproduce with:

```bash
cargo build --release --workspace
bash scripts/benchmark-history-write-ack.bash
```

```text
area=history_write_ack commands=200 p50_us=2412 p95_us=2546 p99_us=2752
```

Script exit status: **1** (budget fail).

Budget comparison:

| Percentile | Result (µs) | Budget (µs) | Pass |
| --- | --- | --- | --- |
| p50 | 2412 | — | — |
| p95 | 2546 | < 2000 | **fail** |
| p99 | 2752 | < 5000 | pass |

Interpretation:

- Samples measure `_mbx_protocol_encode_history_record` through successful
  `_mbx_protocol_decode_history_ack` at the Bash prompt boundary using
  `_mbx_now_us`; they do not include PTY typing/echo, `PS1` render, or SQLite
  commit.
- On this cloud host, p95 exceeds the provisional `HIST-004` write-ack budget
  (p95 < 2 ms, p99 < 5 ms). The benchmark script fails accordingly; the
  documented budget is not weakened.
- Correctness evidence (W-1–W-4) is in `crates/pty/tests/history_write_ack.rs`.
  Foreign-user open (F-1–F-4) is recorded in
  `docs/history-g2-foreign-user-plan.md`.
- Development WSL miss history is preserved in
  `docs/benchmarks/2026-08-16-history-write-ack.md`.

This does not complete `G2`. The write-ack budget remains open.
