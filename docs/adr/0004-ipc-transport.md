# ADR 0004: Use a Bash coprocess for MVP IPC

Status: Accepted for MVP

## Context

The helper can run once per prompt, remain attached as a Bash coprocess, or run as a
Unix-socket daemon. The hot path needs low latency and safe degradation without
complex lifecycle state.

## Decision

Use `coproc` plus the MBX1 stdio protocol by default. Retain process-per-call as the
automatic fallback and development baseline. Keep Unix-socket server support as an
experiment, not the shell's default transport.

The post-hardening release run measured mean PING/PONG time at 1,000 iterations
as 1.068 ms per process, 0.500 ms for the crash-safe Bash coprocess path, and
0.048 ms for a persistent Unix client/server.

## Alternatives

- Per-call execution is simplest but pays startup on every prompt.
- A Unix daemon has warm shared caches and the lowest raw IPC measurement, but Bash
  needs an adapter, and lifecycle/version/socket cleanup become user-visible.

## Consequences

Each Bash session owns one helper and communicates with builtins over file
descriptors. A short Bash subshell isolates each write from SIGPIPE; this is part of
the measured latency. Cross-session caches are unavailable. Broken communication
can be closed locally and fall back immediately.

## Risks

Coprocess FD behavior differs across Bash versions and subshells. A blocked helper
must not stall the prompt, so response acquisition is NUL-aware and byte-capped,
and one absolute render deadline covers request encoding, coprocess exchange,
bounded decode, cleanup, per-call fallback, and the final process-free fallback.
Timed-out child reaping is deferred rather than placing an unbounded `wait` on the
prompt path. Bash enforces the deadline cooperatively between bounded builtin
operations, and process cleanup on shell exit/signals still needs broader PTY and
platform coverage.

## Validation plan

The controlled warm-Git prompt measured p50/p95/p99 of 718/974/1,383 us; see
`docs/benchmarks/2026-08-15-solid-hardening.md`. Extend that evidence to
representative repositories, fallback modes, nested shells, helper crashes, and
supported Bash/platform PTYs. Reconsider a `0600` Unix daemon only if
cross-session history/provider caches produce a measured benefit that covers
adapter complexity.
