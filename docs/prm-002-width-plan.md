# PRM-002 slice: display-width path compaction (W-1–W-6)

Status: `complete` for W-1–W-6 (2026-08-16). Do not mark `PRM-002`, `G0`, `G2`,
or `HIST-007` complete. 16/256/truecolor capability negotiation and PTY
wrap-column probes remain `discovery`. Remaining `G2` is still foreign-user open
and the write-ack budget.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Display-width path compaction (this plan) | `display_path` used `chars().count() > 52`, so 27 East Asian ideographs (54 columns, 27 scalars) stayed uncompacted. This host can prove ASCII/CJK/combining math without PTY wrap probes. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. Do not fake `seteuid`. |
| 3 | Write-ack p95/p99 budget miss | Correctness recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite. |
| — | PRM-002 wrap-column PTY probes / PRM-004 / EDT-001 | Wrap math and representative percentiles stay blocked or discovery. |
| — | HRD-001 macOS PTY matrix | Needs a macOS host. |

## Goal

1. Prompt path compactness uses display columns, not Unicode scalar counts.
2. ASCII, East Asian wide, and combining-mark widths have focused unit tests.
3. Existing 52-column ASCII compaction and home-tilde tests stay green.
4. `PRM-002` stays `discovery` (capability negotiation + wrap-column PTY probes
   remaining). `G0` stays `validation`. Remaining `G2` stays foreign-user open
   and write-ack budget.

## Out of scope (hard)

- 16/256/truecolor capability negotiation
- PTY wrap-column / cursor-position probes
- Readline redisplay or two-line prompt changes
- Retargeting `sanitize_with_limit` from character safety bounds to columns
- Foreign-user open, write-ack product optimization, history storage
- Marking `PRM-002`, `G0`, `G2`, or `HIST-007` complete
- FND-001 CI SHA refresh
- Committing, pushing, or editing shell startup files unless asked

## Cases

| Case | What | Pass |
| --- | --- | --- |
| W-1 | ASCII and empty | `display_width("abc") == 3`; `display_width("") == 0` |
| W-2 | East Asian wide | `display_width("测") == 2`; `display_width("测 试目录") == 9` |
| W-3 | Combining marks | `display_width("e\u{301}") == 1`; `display_width("e\u{301}tude") == 5` |
| W-4 | Wide path compacts on columns | 27 CJK ideographs in `/测测/测测/…` with `chars().count() <= 52` and `display_width > 52` compact via existing `…/parent/leaf` rule |
| W-5 | ASCII threshold unchanged | 52-column ASCII path not compacted; existing long ASCII fixture still `…/characters/project` |
| W-6 | Home tilde unchanged | `injected_home_controls_path_compaction` still yields `~/projects/mbx\n> ` |

## Implementation notes

- `display_width` lives in `crates/cli/src/prompt.rs` and uses the `unicode-width`
  crate because `chars().count()` is the wrong compactness axis for W-4.
- `sanitize_with_limit` remains a **character** safety bound (256 / 1024), not
  display columns (`M-016`).

## Validation (recorded on Linux/WSL)

```bash
cargo test -p mbx --lib prompt -- --nocapture
cargo test -p mbx_pty --test multiline_width wide_glyph combining_mark
cargo clippy --workspace --all-targets -- -D warnings
bash tests/run.bash
```

## Follow-on

- PRM-002 wrap-column PTY probes from `RSH-004` baseline
- PRM-002 16/256/truecolor capability negotiation
- PRM-004 representative/dirty/cold/fallback/platform percentiles
- Remaining `G2`: foreign-user open and write-ack budget
