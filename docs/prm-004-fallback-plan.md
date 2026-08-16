# PRM-004 slice: fallback and Git-disabled prompt percentiles (F-1–F-4)

Status: `deferred` (2026-08-16). Optional later measurement. Unmet or unmeasured
percentile leftovers do not block product development
(`docs/latency-budget-deferral.md`). Do not invent a representative dirty/large
repository.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Fallback + Git-disabled prompt percentiles (this plan) | Optional later measurement only. Timing leftovers no longer block `EDT-001`. |
| 2 | `FND-001` CI SHA refresh | SHA treadmill. Do not spend this packet on it. |
| — | Representative dirty/large/cold/PTY/platform matrix | Needs real repos and macOS. Do not invent a fake repo. |
| — | `EDT-001` / `G3` | Blocked on remaining `G0`. |
| — | Write-ack product chase | `deferred`. Do not reopen unless a test proves SQLite is on the prompt path. |

## Goal

1. Release-mode p50/p95/p99 for Bash fallback prompt rendering (≥1000 iterations).
2. Release-mode p50/p95/p99 for helper prompt with Git disabled (≥1000 warm
   iterations).
3. Compare fallback p99 to the provisional “usable fallback within one 100 ms
   cycle” budget (100000 µs) without weakening it.
4. Keep `PRM-004` `blocked` and `G0` `validation`.

## Out of scope (hard)

- Inventing a representative dirty/large Git repository
- Marking `PRM-004`, `G0`, `G2`, or `HIST-007` complete
- Write-ack product latency or budget changes
- `FND-001` CI SHA refresh
- CPR/DSR / terminal emulator
- Changing `_mbx_fallback_prompt` semantics or `PS1` ownership
- `set -euo pipefail` in sourced Bash modules
- Committing, pushing, or editing shell startup files unless asked

## Method

Time **render only**, not PTY typing/echo.

Reuse the `now_us` / `percentile` style from `scripts/benchmark-prompt.bash`
(`EPOCHREALTIME` microseconds). Do not add a new clock.

### F-2 Bash fallback

Source only what the existing module tests source for the renderer:

- `bash/protocol.bash` (flag constants)
- `bash/fallback.bash`

Do **not** source `init.bash` into the benchmark process as a user’s shell.
Do **not** write `PS1` (`_mbx_fallback_prompt` returns through `REPLY`).

Fixed context (already asserted in `tests/bash/modules.bash`):

```bash
_mbx_fallback_prompt 0 - /tmp "$_MBX_FLAG_NO_COLOR"
```

Discard `REPLY` after each call. Warm once, then sample ≥1000 iterations.
Print:

```text
workload=fallback-prompt iterations=… p50_us=… p95_us=… p99_us=…
```

Fail the **script** (not `tests/run.bash`) if `p99_us >= 100000`.

### F-3 Git-disabled helper prompt

Sibling of `scripts/benchmark-prompt.bash`:

- `cargo build --release --workspace` (or require `target/release/mbx`)
- `mbx serve --stdio` coprocess
- MBX1 `PROMPT` with `cwd=/tmp` (not a Git repo), `status=0`, `duration=-`
- flags = `_MBX_FLAG_NO_COLOR | _MBX_FLAG_ASCII_ICONS | _MBX_FLAG_DISABLE_GIT`
  (same additive bits as protocol.bash; do not invent new bits)
- helper env: `MBX_DISABLE_GIT=1`
- warm one request, then ≥1000 iterations
- do not `git init`

Print:

```text
workload=git-disabled-prompt iterations=… p50_us=… p95_us=… p99_us=…
```

This is **not** the representative dirty-repo case.

## Test cases

| ID | Case | Assert |
| --- | --- | --- |
| F-1 | Harness | `scripts/benchmark-prompt-fallback.bash` exists; builds/uses release `mbx` for F-3; prints one `workload=` line per workload; budgets 100000 µs on fallback p99 only |
| F-2 | Bash fallback | ≥1000 iterations of `_mbx_fallback_prompt 0 - /tmp "$_MBX_FLAG_NO_COLOR"`; p50/p95/p99 recorded; script fails if p99 ≥ 100000 µs |
| F-3 | Git-disabled helper | ≥1000 warm MBX1 PROMPT rounds with `MBX_DISABLE_GIT=1` and no `git init`; p50/p95/p99 recorded |
| F-4 | Docs | `docs/benchmarks/2026-08-16-prompt-fallback.md` with host facts; roadmap / architecture note fallback percentiles recorded; `PRM-004` stays `blocked`; `G0` stays `validation`; one changelog row |

## Remaining `G0`

Platform matrix, `HRD-001` macOS PTY run, representative dirty/large/cold/PTY
`PRM-004` percentiles.

## Remaining `G2`

None required. Write-ack percentiles are `deferred`.
