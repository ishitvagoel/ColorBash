# HIST-007 slice: prompt-boundary write-ack cloud remeasure (W-5)

Status: `complete` for C-1–C-4 (2026-08-16). Cloud release percentile run
**misses** the provisional write-ack budget (p95). Do not mark `G2` or
`HIST-007` complete. Do not change product code or weaken budgets.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Write-ack cloud remeasure (this plan) | W-1–W-4 correctness recorded; foreign-user open recorded. Only remaining `G2` evidence is the write-ack percentile budget. This cloud host can re-run the existing release PTY harness with no product changes. |
| 2 | `FND-001` CI SHA refresh to `0bc60e6` | Docs-only; green CI already exists. |
| — | PRM-002 wrap-column PTY probes | Raw PTY is not an emulator; do not hang on DSR. |
| — | `HRD-001` macOS PTY matrix | Needs macOS host. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked on remaining `G2` / `G3` / `G0` matrix |

## Goal

1. Re-run `scripts/benchmark-history-write-ack.bash` on this cloud host in
   release mode with ≥200 admitted commands, production 0.10 s timeouts,
   `MBX_DISABLE_GIT=1`, and M-019 `wait_all` sync.
2. Record p50/p95/p99 and compare to p95 < 2000 µs and p99 < 5000 µs without
   weakening the budget.
3. If percentiles pass, close the write-ack leftover and mark `G2` /
   `HIST-007` complete only when no other `G2` evidence remains. If they miss,
   record the cloud miss and keep `G2` / `HIST-007` at `validation`.
4. Leave product code untouched.

## Out of scope (hard)

- Changing Rust, Bash product modules, `WRITER_BATCH_SIZE`, `wait_for_count`, ACK
  meaning, MBX2, or MBX1 framing
- Weakening the provisional budget (p95 < 2000 µs, p99 < 5000 µs)
- Chasing product-code latency on a miss
- `FND-001` CI SHA refresh
- PRM-002 wrap-column / DSR probes
- Reintroducing command-text diagnostics (`M-023`)
- Marking `G2` or `HIST-007` complete on a miss
- Committing, pushing, or editing shell startup files unless asked

## Test cases

| ID | Case | Assert | Result |
| --- | --- | --- | --- |
| C-1 | Harness unchanged: script builds release `mbx`, drives `measure_prompt_boundary_write_ack_percentiles --ignored`, budgets 2000 / 5000 µs | Script and ignored test unchanged | **pass** |
| C-2 | Cloud evidence file with environment, reproduce commands, p50/p95/p99, pass/fail vs budget; WSL benchmark doc links cloud remeasure and notes foreign-user recorded | `docs/benchmarks/2026-08-16-history-write-ack-cloud.md` created; WSL miss preserved | **pass** |
| C-3 | Pass path: p95 < 2000 and p99 < 5000 → close write-ack leftover; `G2` / `HIST-007` complete only when exit criteria fully met | N/A — percentiles miss | **not applicable** |
| C-4 | Miss path: keep `G2` / `HIST-007` at `validation`; document cloud miss; no product changes; no budget weakening | p95=2546 µs ≥ 2000 µs; script exit 1 | **pass** |

## Remaining `G2`

Write-ack percentile budget (p95 miss on development WSL and this cloud host).
Foreign-user open is recorded.

## Remaining `G0`

Platform matrix, `HRD-001` macOS PTY run, `PRM-004` representative percentiles.
`PRM-002` stays `discovery` (wrap-column PTY probes remaining).
