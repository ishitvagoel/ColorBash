# GHST-001 leftover: ghost QUERY with generation stale rejection

Status: `complete` (2026-08-25). Coprocess QUERY, client generation checks,
overlapping delayed-RESULT PTY, and CANCEL-after-QUERY prompt survival are
recorded. Dim paint / overlay stay `deferred`. Sync CLI search remains the
fallback when no coprocess is attached.

## Why this slice

QUERY/RESULT/CANCEL wire is landed. Ghost still forked `mbx history search`.
ADR 0011 makes stale rejection a Bash duty. Sequential coprocess exchange is
1:1 (ADR 0011 HOL policy); generation still must be ignored when it is not
current. Bind `-x` blocks until the handler returns, so extra typed bytes sit
in the TTY queue; a helper that withholds the first RESULT until a second
QUERY arrives is the overlapping delayed-RESULT case.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Coprocess QUERY + generation check | Unblocks ghost from per-key CLI forks. |
| 2 | Overlapping delayed-RESULT PTY (this leftover) | Hanging/hold helper plus skip-stale read. |
| — | Dim paint / overlay | `deferred`. |

## Goal

1. When the coprocess is ready, ghost sends `QUERY` `prefix` with a monotonic
   generation and applies RESULT only if `generation == _MBX_GHOST_GENERATION`.
2. Timed-out QUERY **read** leaves the engine running so a delayed RESULT can
   be skipped on the next QUERY. A failed write or an unexpected non-RESULT
   frame still stops the engine (same desync rule as RECORD for hard failures).
3. Without a coprocess, keep the existing CLI prefix search (module stubs).
4. CANCEL after QUERY must leave a later usable prompt.

## Asserts

| ID | Evidence |
| --- | --- |
| W-1 | Module: current generation fills candidates; older generation does not |
| W-2 | History+ghost PTY: type a short prefix, then a longer non-matching prefix before RESULT 1 arrives; stale generation does not change `READLINE_LINE`; matching generation may show a suffix (`crates/pty/tests/ghost.rs` `overlapping_delayed_result_is_rejected`) |
| W-3 | CLI fallback still restores `set -m` (existing contract) |
| W-4 | CANCEL after QUERY leaves a usable prompt (`crates/pty/tests/ghost.rs` `cancel_after_query_leaves_usable_prompt`) |

## Stop

Do not start highlighting or overlay. Do not start the next ranked leftover
in the same change.
