# PRM-002 slice: wrap-column PTY discovery (W-C-1–W-C-4)

Status: `complete` for W-C-1–W-C-4 (2026-08-16). Do not mark `G0`, `G2`, or
`HIST-007` complete. `PRM-002` moves to `validation` with non-DSR wrap evidence
recorded; representative `PRM-004` percentiles and `HRD-001` macOS matrix remain
`G0` work.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Wrap-column PTY discovery (this plan) | Redirected-output, display-width, and color capability are recorded. This host can extend `RSH-004` with non-DSR wrap usability probes at known `COLUMNS` without a terminal emulator. |
| 2 | Write-ack p95/p99 budget miss | W-1–W-4 correctness recorded; WSL and cloud remeasure miss. Do not chase product-code latency unless a test proves the prompt waits on SQLite. |
| — | `HRD-001` macOS PTY matrix | Needs a macOS host. |
| — | `PRM-004` representative percentiles | Blocked on representative workloads; separate from wrap discovery. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked on remaining `G2` / `G3` / `G0` matrix |

## Goal

1. Document that CPR/DSR (`\e[6n`) cannot be asserted on a raw PTY harness; no test
   may block waiting for a cursor-position reply.
2. Add focused non-DSR PTY wrap probes at known `COLUMNS`: typed input longer than
   the window width and wide-glyph payloads still execute with a usable next prompt.
3. Reuse `crates/pty/tests/multiline_width.rs` and M-019-safe `wait_all` sync.
4. `PRM-002` moves to `validation` (wrap discovery closed). `G0` stays
   `validation`. `G2` / `HIST-007` stay `validation` for write-ack budget.

## Out of scope (hard)

- Sending `\e[6n` / DSR and waiting for CPR
- A terminal emulator or CPR responder
- Byte-level exact wrap-column math or pixel assertions
- Marking `G0`, `G2`, or `HIST-007` complete
- Write-ack product optimization or budget weakening
- `FND-001` CI SHA refresh
- Committing, pushing, or editing shell startup files unless asked

## CPR/DSR policy (W-C-1)

A raw Linux PTY pair has no cursor-position responder. Sending Device Status
Report `\e[6n` and waiting for a CPR reply will hang indefinitely. The MBX PTY
harness therefore **forbids** CPR/DSR waits. Wrap evidence is limited to
usability at known `WinSize.cols` / `COLUMNS`: exact command output and a usable
next `> ` prompt.

## Test cases

| ID | Case | Assert | Result |
| --- | --- | --- | --- |
| W-C-1 | CPR/DSR policy documented | Plan + `docs/research/multiline-width-pty.md` forbid `\e[6n` waits; no test blocks on CPR | **pass** |
| W-C-2 | Narrow wrap usability (non-DSR) | `narrow_wrap_long_command_stays_usable`: `cols=20`, command longer than window; exact output + next `> ` | **pass** |
| W-C-3 | Wide-glyph wrap usability (non-DSR) | `narrow_wrap_wide_glyph_payload_stays_usable`: `cols=12`, CJK path exceeds window; command executes + next `> ` | **pass** |
| W-C-4 | Docs closeout | Research note, roadmap, architecture updated; `PRM-002` → `validation`; G0/G2 unchanged for write-ack | **pass** |

## Remaining `G2`

Write-ack percentile budget (WSL + cloud remeasure miss). Foreign-user open
recorded.

## Remaining `G0`

Platform matrix, `HRD-001` macOS PTY run, `PRM-004` representative percentiles.
