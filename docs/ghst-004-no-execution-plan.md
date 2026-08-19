# GHST-004 partial: no-execution PTY evidence (C-1–B-1)

Status: `validation` for Ctrl+C and bracketed-paste cases (2026-08-19). Latency
matrix and full `GHST-004` exit remain. Do **not** mark `GHST-004` complete.

## Why this slice

Resize, exact-byte, and PS2 continuation are recorded in
`docs/ghst-004-multiline-resize-plan.md`. `GHST-004` still needs no-execution
evidence beyond ordinary typing. Dim paint and async stale-rejection stay blocked.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Ctrl+C + bracketed paste (this plan) | Runnable on the Linux PTY harness. |
| — | Latency p99 matrix | `deferred` per `docs/latency-budget-deferral.md`. |
| — | Dim paint / async stale rejection | Blocked on decoration / IPC ADR. |

## Goal

1. With an active ghost suffix, `\C-c` does not execute the suggestion; the next
   prompt accepts a new command.
2. Bracketed paste bytes that complete a prefix may show a suffix; they do not
   execute until Enter.
3. Do not mark `GHST-004` complete. Do not start highlighting or overlay.

## Out of scope (hard)

- Dim ANSI, GUI overlay, syntax highlighting
- Async generation IDs (`GHST-001`)
- Latency percentile benches
- Marking Phase 4 or `GHST-004` complete

## Asserts

| ID | Evidence |
| --- | --- |
| C-1 | History+ghost PTY: record `echo MBX_GHST:alpha`; type `echo MBX_GHST:a`; suffix visible; `\C-c`; next command prints `MBX_GHST:cancelled` and not `MBX_GHST:alpha` |
| B-1 | History+ghost PTY: record `echo MBX_GHST:alpha`; type `echo `; bracketed-paste `MBX_GHST:a`; suffix visible; Enter prints `MBX_GHST:a` not `MBX_GHST:alpha` |

## Stop

Do not mark `GHST-004` complete. Do not start highlighting or overlay.
