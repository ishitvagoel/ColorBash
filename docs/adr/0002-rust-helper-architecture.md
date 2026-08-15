# ADR 0002: A small Rust helper supports Bash

Status: Accepted for the foundation

## Context

Prompt rendering, indexing, ranking, caching, provider work, and terminal-safe
string handling will exceed what should run in prompt Bash. The foundation needs a
native handshake without committing to heavyweight dependencies.

## Decision

Use a Rust workspace with a protocol library and one `mbx` binary. Keep the Bash
layer understandable and execution-free. The foundation uses the standard library
only; crates must be justified by measured need. Long-lived helper processes may
cache future metadata, but Bash remains functional without them.

## Alternatives

- Pure Bash was rejected for future indexing/ranking and structured UI latency.
- Python/Node helpers were rejected for startup/runtime variability in a shell hot
  path.
- A large daemon/framework was deferred until workload evidence exists.

## Consequences

The prototype has one optimized binary, predictable memory safety, and a typed
protocol. Contributors need a Rust toolchain; Git discovery still invokes `git`.

## Risks

Cross-platform terminal behavior, binary distribution, process lifecycle, and a
growing dependency graph could complicate installation.

## Validation plan

Build on Linux, WSL, and macOS; measure release startup and resident process
latency; test helper crash/restart; require benchmarks and ADR updates before major
runtime dependencies.

