# SOLID hardening benchmark — 2026-08-15

Environment: optimized `mbx 0.1.0`, x86_64 WSL2/Linux 6.6, Bash 5.2.21,
Rust 1.85.1, and Git 2.43.0. Each workload used 1,000 warm iterations. The
Unix-socket workload ran outside the restricted development sandbox because the
sandbox denies local socket binding.

## Warm prompt with Git

The prompt workload starts one stdio server, creates an empty Git repository,
warms the repository-status cache, then measures complete MBX1 prompt request and
response round trips over the retained coprocess descriptors.

```text
workload=warm-prompt-git iterations=1000 p50_us=718 p95_us=974 p99_us=1383
```

This satisfies the provisional cached-prompt targets (p95 at most 10 ms and p99
at most 25 ms) for this controlled repository. It is not the full `PRM-004` or
`G0` matrix: large/dirty repositories, cold refreshes, PTY lifecycle, supported
platforms, terminal widths, and fallback modes still need representative
percentile evidence.

## IPC comparison

```text
transport=process-per-call iterations=1000 total_ns=1068497863 mean_ns=1068497
transport=bash-coprocess iterations=1000 total_ns=500066277 mean_ns=500066
transport=unix-socket iterations=1000 total_ns=47531250 mean_ns=47531
```

These are PING/PONG transport microbenchmarks. The guarded coprocess result
includes Bash's SIGPIPE-isolating write subshell. The Unix-socket result uses a
persistent Rust client and does not represent a native Bash integration.

## Reproduce

```bash
cargo build --release --workspace
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-prompt.bash target/release/mbx
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-ipc.bash target/release/mbx
```
