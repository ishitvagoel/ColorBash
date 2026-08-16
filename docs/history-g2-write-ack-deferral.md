# HIST-007 / G2: defer write-ack percentile leftover

Status: `deferred` (2026-08-16). Authorized close of `G2` without a write-ack
percentile pass. The provisional budget is **not** weakened and is **not**
recorded as met.

## Decision

Functionally, prompt-boundary write-ack is correct (W-1–W-4). ACK means the
helper accepted the record onto the bounded queue; W-3 shows samples at prompt
return before SQLite drain. Storage failure and queue saturation still degrade
without breaking the shell.

Release PTY percentiles miss the provisional `HIST-004` write-ack budget on
development WSL and on the 2026-08-16 cloud remeasure (cloud p95=2546 µs vs
p95 < 2000 µs). Product-code latency chasing remains forbidden unless a new
test proves the prompt waits on SQLite.

The remaining percentile leftover is moved to `deferred` so `G2` / `HIST-007`
can close. Revisit later; do not treat this as a budget pass.

## What stays true

| Item | Status |
| --- | --- |
| W-1–W-4 correctness | recorded |
| WSL miss (`docs/benchmarks/2026-08-16-history-write-ack.md`) | preserved |
| Cloud miss (`docs/benchmarks/2026-08-16-history-write-ack-cloud.md`) | preserved |
| Budget p95 < 2000 µs, p99 < 5000 µs | unchanged |
| `scripts/benchmark-history-write-ack.bash` fail-on-miss | unchanged |
| Capture default-off (`MBX_HISTORY=1`) | unchanged |

## Revisit when

- a host/run meets both percentiles with the existing harness, or
- a new test proves SQLite (or another defect) is on the prompt path, or
- an accepted ADR ratifies or changes the write-ack budget.

## Out of scope (this change)

- Product-code latency work
- Weakening or deleting the budget
- Enabling capture by default
- Marking the write-ack budget `complete`
