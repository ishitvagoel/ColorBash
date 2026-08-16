# History write-ack benchmark — 2026-08-16

Environment: release `mbx` PTY benchmark via
`crates/pty/tests/history_write_ack.rs`, x86_64 WSL2/Linux 6.6.87,
Bash 5.2.21, rustc 1.97.1, load average 1.80 / 1.39 / 1.26. Production
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
area=history_write_ack commands=200 p50_us=4036 p95_us=6362 p99_us=7244
```

Earlier runs on this host used a stale `$ROOT/target/release/mbx` while cargo
wrote to `CARGO_TARGET_DIR` (p50≈4673–4805, p95≈6451–7672, p99≈7470–10857).
Percentiles vary run to run; all recorded runs miss the provisional budget.

Interpretation:

- Samples measure `_mbx_protocol_encode_history_record` through successful
  `_mbx_protocol_decode_history_ack` at the Bash prompt boundary using
  `_mbx_now_us`; they do not include PTY typing/echo, `PS1` render, or SQLite
  commit.
- On this WSL host, p95/p99 exceed the provisional `HIST-004` write-ack budget
  (p95 < 2 ms, p99 < 5 ms). The benchmark script fails accordingly; the
  documented budget is not weakened.
- Correctness evidence (W-1–W-4) is in `crates/pty/tests/history_write_ack.rs`:
  opt-in sampling, digit-only lines, no command text in the sample file, `0600`
  sample-file mode, samples present at prompt return before SQLite drain, and
  empty Enter produces no sample.

Foreign-user open (F-1–F-4) is recorded in `docs/history-g2-foreign-user-plan.md`.
Cloud remeasure on 2026-08-16 also misses the write-ack budget (p95=2546 µs);
see `docs/benchmarks/2026-08-16-history-write-ack-cloud.md`.

This does not complete `G2`. The write-ack budget remains.
WAL crash/corrupt recovery, WAL/SHM `0600` never-more-permissive, and many-match
prefix covering-index evidence are recorded in `crates/cli/src/storage.rs` and
`docs/benchmarks/2026-08-16-history-prefix.md`.
