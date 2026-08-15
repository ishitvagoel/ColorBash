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

Measured mean PING/PONG time at 1,000 iterations was 1.288 ms per process, 0.573 ms
for the crash-safe Bash coprocess path, and 0.060 ms for a persistent Unix
client/server.

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
could stall the prompt, so every read is deadline-bounded. Process cleanup on shell
exit and signals needs broader PTY coverage.

## Validation plan

Benchmark complete prompt p50/p95/p99, test nested shells and helper crashes, and
verify Bash 5.x/macOS behavior. Reconsider a `0600` Unix daemon only if cross-session
history/provider caches produce a measured benefit that covers adapter complexity.
