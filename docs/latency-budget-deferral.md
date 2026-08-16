# Policy: unmet percentile targets do not block product development

Status: `accepted` (2026-08-16). Authorized by the product owner.

## Decision

Provisional p50/p95/p99 budgets are **review leftovers**, not development gates.

If a percentile target is missed, or a representative timing matrix is not yet
measured:

- record the miss or the gap;
- move that leftover to `deferred`;
- **do not** weaken the documented numbers;
- **do not** chase product-code latency unless a test proves a functional
  prompt-path defect (for example, the prompt waits on SQLite);
- continue the next product slice.

Correctness, Bash compatibility, and “never break the shell” still block.

## Applied now

| Leftover | Status | Notes |
| --- | --- | --- |
| History write-ack p95/p99 | `deferred` | Already recorded; `docs/history-g2-write-ack-deferral.md` |
| `PRM-004` full prompt percentile matrix (fallback, dirty/large, cold, PTY, platform) | `deferred` | Controlled warm-Git case remains on file |
| `docs/prm-004-fallback-plan.md` | `deferred` | Optional later measurement; not Immediate next work |

## What this is not

- Not a budget pass
- Not permission to enable capture by default
- Not a close of `HRD-001` (macOS/platform matrix is not a timing leftover)
- Not a change to ACK meaning, MBX1/MBX2, or Readline ownership

## Revisit

Before `G5` / release hardening, review deferred percentile leftovers and either
meet them, ratify new numbers in an ADR, or keep them explicitly out of MVP
scope.
