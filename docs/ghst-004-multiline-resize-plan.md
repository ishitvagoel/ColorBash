# GHST-004 partial: resize and exact-byte PTY evidence (R-1–Q-1)

Status: `validation` for resize and quoted-prefix cases (2026-08-19). Latency
matrix and full `GHST-004` exit remain. Do **not** mark `GHST-004` complete.

## Why this slice

`GHST-002` motion and kill-ring slices are recorded. Dim paint and async lookup
stay blocked. `GHST-004` needs PTY evidence that ghost suffix behavior survives
resize and quoting without auto-executing suggestions.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Resize + quoted exact bytes (this plan) | Runnable on the Linux PTY harness. |
| — | Multiline PS2 matrix | Follow-on once quoted/resize are stable. |
| — | Latency p99 matrix | `deferred` until functional cases are recorded. |
| — | Dim paint / async stale rejection | Blocked on decoration / IPC ADR. |

## Goal

1. With an active ghost suffix, `SIGWINCH` resize does not auto-execute the
   suggestion or break Enter discard of the unaccepted suffix.
2. A spaced multi-token history row preserves exact typed bytes at the prefix
   boundary; Enter still admits only the typed prefix, not the full history row.
3. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

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

## Stop

Do not mark `GHST-004` complete. Do not start highlighting or overlay.
