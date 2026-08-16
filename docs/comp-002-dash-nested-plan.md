# COMP-002 slice: `--` and nested insertion (N-1–N-2)

Status: `ready` (2026-08-16). L-1–L-4 are in `validation`. This packet proves
two remaining G4 insertion contexts on stock file completion after default
MBX install: a filename after `--`, and a filename inside `$(...)`.
Do not mark `G4` or `COMP-002` complete.

Slow/stateful fallthrough and the 5 ms adapter budget are a later leftover.
Do not start them here.

## Why this slice

Immediate next work after L-1–L-4. ADR 0006 / G4 still name `--` and nested
commands. Same method as L-1–L-4: fixture-off, unwrapped file completion,
`printf 'GOT:%s|\n'` observation.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `--` and nested insertion (this plan) | Named G4 contexts still without PTY bytes. |
| 2 | Slow/stateful fallthrough | Different seam (`_mbx_comp_wrap_existing_f`); start after N-1–N-2. |
| — | Adapter 5 ms budget | `deferred` with other percentiles. |
| — | Popup / ranking / Git candidates | Blocked on `G4`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. After default `init.bash` install (no `MBX_COMP_FIXTURES`), Tab on
   `MBX_COMP_U` still inserts `MBX_COMP_UNIQUE` when the word follows `--`
   and when the word is inside `$(...)`.
2. `ls` and `printf` stay unwrapped. No new global `complete` specs.
3. `COMP-002` stays `validation`. `G4` stays `discovery`.

## Out of scope (hard)

- Slow or stateful completion fallthrough
- Completion adapter overhead / 5 ms latency budget
- Pipelines, `` `...` ``, or a second nested form
- Popup, ranking, descriptions, Git candidates (`COMP-003`+)
- Wrapping `ls`, `printf`, or default file completion
- Rebinding printable keys or taking Readline ownership
- Auto-executing completed text
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Percentile benches, write-ack, FND-001 SHA refresh
- Marking `G4`, `G3`, `EDT-001`, `COMP-001`, or `COMP-002` complete
- Rewriting this plan's cases or adding extra filenames
- Committing unless asked

## Method

Keep stock completion authoritative (ADR 0006). Do not call
`_mbx_comp_wrap_existing_f` on `ls` or `printf`. Use
`crates/pty/tests/completion_harness.rs` and `spawn_mbx_shell` **without**
`MBX_COMP_FIXTURES`. Tab is `0x09`. M-019: `wait_all` for the GOT line plus
`> `. Do not wait on CPR/DSR. Do not log command text (M-023).

If a measured GOT line differs from the expected column, keep the host's
stock bytes and record them in this plan's Status cell. Do not normalize
quoting.

Each case uses one isolated `TempHome` and only file `MBX_COMP_UNIQUE`.

## Test cases

| ID | Case | Setup | Type, then Tab, then Enter | Expected GOT | Status |
| --- | --- | --- | --- | --- | --- |
| N-1 | Filename after `--` | File `MBX_COMP_UNIQUE`. | `printf 'GOT:%s\|\n' -- MBX_COMP_U` Tab Enter | `\nGOT:MBX_COMP_UNIQUE\|` then `> ` | validation — `double_dash_file_completion_preserves_stock_bytes` |
| N-2 | Filename inside `$(...)` | File `MBX_COMP_UNIQUE`. | `: $(printf 'GOT:%s\|\n' MBX_COMP_U` Tab, then `)` Enter | `\nGOT:MBX_COMP_UNIQUE\|` then `> `. Do not leave the shell on `PS2`. | validation — `nested_substitution_file_completion_preserves_stock_bytes`; observation uses `echo $(printf ...)` because `:` captures printf stdout in substitution |

### N-1 notes

`--` is an argument separator, not a command. Do not complete the word `--`
itself. The unique-file prefix is the same as P-1 / L-1 so the GOT line is
comparable. `printf` stays unwrapped.

### N-2 notes

The outer `:` discards the substitution result so only the inner `printf`
prints GOT. After Tab, type `)` then Enter. If Tab already closed `$(...)`,
Enter only — record that in the Status cell. If stock opens `PS2`, finish
the `)` and treat a missing GOT as a defect, not as a reason to drop N-2.

## Module contract (no new cases)

Keep the existing F-1 / F-2 / probe / flag / L-1–L-4 asserts. Add only:

- `complete -p ls` and `complete -p printf` still contain no `_mbx_comp`
- default install still defines no `mbx_comp_*`

Do not add `--` or `$(...)` logic to `bash/completion.bash` unless a test
proves the adapter mutates these lines today. The expected outcome is **no
product change** beyond tests and this plan's status column.

## Remaining after this slice

Slow/stateful fallthrough through `_mbx_comp_wrap_existing_f`, and the
provisional 5 ms adapter overhead budget. Popup stays blocked until `G4`
passes.
