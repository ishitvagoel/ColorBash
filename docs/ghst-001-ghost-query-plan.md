# GHST-001 leftover: ghost QUERY with generation stale rejection

Status: `validation` for coprocess QUERY + client generation check (2026-08-25).
Overlapping delayed-RESULT PTY remains. Do **not** mark `GHST-001` complete.

## Why this slice

QUERY/RESULT/CANCEL wire is landed. Ghost still forked `mbx history search`.
ADR 0011 makes stale rejection a Bash duty. Sequential coprocess exchange is
1:1 (ADR 0011 HOL policy); generation still must be ignored when it is not
current.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Coprocess QUERY + generation check (this plan) | Unblocks ghost from per-key CLI forks. |
| 2 | Overlapping delayed-RESULT PTY | Needs a hanging helper; later leftover. |
| — | Dim paint / overlay | `deferred`. |

## Goal

1. When the coprocess is ready, ghost sends `QUERY` `prefix` with a monotonic
   generation and applies RESULT only if `generation == _MBX_GHOST_GENERATION`.
2. Timed-out QUERY exchange stops the engine (same desync rule as RECORD).
3. Without a coprocess, keep the existing CLI prefix search (module stubs).
4. Do not mark `GHST-001` complete.

## Asserts

| ID | Evidence |
| --- | --- |
| W-1 | Module: current generation fills candidates; older generation does not |
| W-2 | Existing ghost PTY G-1 still shows suffix via coprocess QUERY |
| W-3 | CLI fallback still restores `set -m` (existing contract) |

## Stop

Do not start highlighting or overlay. Do not mark `GHST-001` or `GHST-004`
complete.
