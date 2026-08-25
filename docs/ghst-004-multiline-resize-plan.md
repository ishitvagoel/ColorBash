# GHST-004 partial: resize, exact-byte, and PS2 PTY evidence (R-1–M-1)

Status: `complete` for resize, spaced exact-byte, and PS2 continuation
(2026-08-19). Functional `GHST-004` editing/safety evidence is recorded with
Ctrl+C and bracketed paste (`docs/ghst-004-no-execution-plan.md`). Latency
percentiles stay `deferred` (`docs/latency-budget-deferral.md`) and are not a
Phase 4 exit. Dim paint / overlay stay `deferred`.

## Why this slice

`GHST-002` motion and kill-ring slices are recorded. Dim paint and async lookup
stay blocked. `GHST-004` needs PTY evidence that ghost suffix behavior survives
resize, prefix-boundary exact bytes, and PS2 continuation without auto-executing
suggestions.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Resize + exact bytes + PS2 (this plan) | Runnable on the Linux PTY harness. |
| — | Latency p99 matrix | `deferred` until functional cases are recorded. |
| — | Dim paint / async stale rejection | Blocked on decoration / IPC ADR. |

## Goal

1. With an active ghost suffix, `SIGWINCH` resize does not auto-execute the
   suggestion or break Enter discard of the unaccepted suffix.
2. A spaced multi-token history row preserves exact typed bytes at the prefix
   boundary; Enter still admits only the typed prefix, not the full history row.
3. On a `PS2` continuation line, an active suffix is discarded on Enter; the
   folded command uses the typed continuation prefix, not the unaccepted suffix.
4. Dim paint and overlay stay `deferred`. Latency percentiles stay `deferred`.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Async generation IDs (`GHST-001`)
- Latency percentile benches
- Marking Phase 4 or `GHST-004` complete

## Asserts

| ID | Evidence |
| --- | --- |
| R-1 | History+ghost PTY: record `echo MBX_GHST:alpha`; type `echo MBX_GHST:a`; resize; Enter prints `MBX_GHST:a` not `MBX_GHST:alpha` |
| Q-1 | History+ghost PTY: record `echo MBX_GHST:alpha beta`; type `echo MBX_GHST:alp`; suffix visible; Enter prints `MBX_GHST:alp` not `MBX_GHST:alpha` |
| M-1 | History+ghost PTY with `PS2=CONT> `: record `echo MBX_GHST:cont`; type `echo one \` then on PS2 `echo MBX_GHST:c`; suffix visible; Enter prints `one echo MBX_GHST:c` not `one echo MBX_GHST:cont` |

## Stop

Do not start highlighting or overlay. Latency percentiles stay `deferred`.
