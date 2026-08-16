# COMP-002 slice: leftover insertion matrix (L-1–L-4)

Status: `validation` (2026-08-16). `COMP-001` / `COMP-002` are in `validation`.
L-1–L-4 evidence is in `crates/pty/tests/completion_harness.rs`. Do not mark
`G4` or `COMP-002` complete.

`--`, nested commands, slow/stateful fallthrough, and the 5 ms adapter
budget are a later leftover. Do not start them here.

## Why this slice

Immediate next work. ADR 0006 still requires aliases, redirections, Unicode,
and incomplete quotes before a popup. The inspect-before-wrap seam exists;
these four cases do not need a new wrap. File completion stays stock.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Leftover insertion matrix (this plan) | Named G4 contexts still without PTY bytes. |
| 2 | `--` / nested / slow fallthrough | Second leftover; start only after L-1–L-4. |
| — | Adapter 5 ms budget | `deferred` with other percentiles. |
| — | Popup / ranking / Git candidates | Blocked on `G4`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. After default `init.bash` install (no `MBX_COMP_FIXTURES`), Tab on a unique
   filename still inserts the same ordinary Bash bytes as stock when the word
   is reached through an alias, a redirection target, a Unicode name, or an
   incomplete single quote.
2. `ls` and `printf` stay unwrapped. No new global `complete` specs.
3. `COMP-002` stays `validation`. `G4` stays `discovery`.

## Out of scope (hard)

- `--` as a completion word, nested `$(...)` / pipelines
- Slow or stateful completion fallthrough
- Completion adapter overhead / 5 ms latency budget
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
`_mbx_comp_wrap_existing_f` on `ls`, `printf`, or the alias name. These cases
are unwrapped file completion under an installed (fixture-off) adapter.

Reuse `crates/pty/tests/completion_harness.rs`. Use `spawn_mbx_shell` **without**
`MBX_COMP_FIXTURES` so L-1–L-4 match a normal user session. Tab is `0x09`.
M-019: `wait_all` for the GOT line plus `> `. Do not wait on CPR/DSR. Do not
log command text (M-023).

Observe through `printf 'GOT:%s|\n'`. If a measured GOT line differs from the
expected column below, keep the host's stock bytes and record them in this
plan's status cell. Do not normalize quoting.

Each case uses one isolated `TempHome` and only the sentinel file named in
that row.

## Test cases

| ID | Case | Setup | Type, then Tab, then Enter | Expected GOT (this host, from P-1 style) | Status |
| --- | --- | --- | --- | --- | --- |
| L-1 | Alias | File `MBX_COMP_UNIQUE`. After prompt: `alias mbxpr=printf` then Enter, wait `> `. | `mbxpr 'GOT:%s\|\n' MBX_COMP_U` | `\nGOT:MBX_COMP_UNIQUE\|` then `> ` | `validation` — `alias_file_completion_preserves_stock_bytes` |
| L-2 | Redirection | File `MBX_COMP_UNIQUE` present (default filename completion on this host requires an existing unique match before Tab can expand the redirect target). | `printf 'x' > MBX_COMP_U` Tab Enter. Then `printf 'GOT:%s\|\n' MBX_COMP_*` Enter. | After the second command: `\nGOT:MBX_COMP_UNIQUE\|` then `> `. | `validation` — `redirection_target_completion_preserves_stock_bytes` |
| L-3 | Unicode filename | File `MBX_COMP_café` only (NFC). | `printf 'GOT:%s\|\n' MBX_COMP_c` | `\nGOT:MBX_COMP_café\|` then `> ` | `validation` — `unicode_filename_completion_preserves_stock_bytes` |
| L-4 | Incomplete single quote | File `MBX_COMP_UNIQUE`. | `printf 'GOT:%s\|\n' 'MBX_COMP_U` Tab, then Enter (Tab closes the quote on this host). | `\nGOT:MBX_COMP_UNIQUE\|` then `> `. Do not leave the shell on `PS2`. | `validation` — `incomplete_quote_file_completion_preserves_stock_bytes` |

### L-1 notes

`mbxpr` is an alias for `printf`, not an MBX fixture. Do not `complete -F` it.
The unique-file prefix is the same as P-1 so the GOT line is comparable.

### L-2 notes

The completed word is the redirect target. On this host, default filename
completion requires an existing `MBX_COMP_UNIQUE` before Tab can expand
`MBX_COMP_U`. The redirect then truncates/overwrites that file. The second
command prints the glob match. Do not also create a partial `MBX_COMP_U` file
in setup.

### L-3 notes

One Unicode sentinel only. Do not add a second glyph, emoji, or combining-mark
file. `LANG` / `LC_ALL` stay `C.UTF-8` as in the existing spawn helper.

### L-4 notes

The typed prefix includes an opening `'` and no closing `'`. On this host, Tab
completes inside the quote and closes it (`'MBX_COMP_UNIQUE'`). Enter
immediately; do not add another `'`. If stock opens `PS2`, that is a failed
case.

## Module contract (no new cases)

Keep the existing F-1 / F-2 / probe / flag asserts. Add only:

- `complete -p ls` and `complete -p printf` still contain no `_mbx_comp`
- default install (unset `MBX_COMP_FIXTURES`) still defines no `mbx_comp_*`

Do not add alias/Unicode logic to `bash/completion.bash` unless a test proves
the adapter mutates these lines today. The expected outcome is **no product
change** beyond tests and this plan's status column.

## Remaining after this slice

`--`, nested commands, slow/stateful fallthrough, and the provisional 5 ms
adapter overhead budget. Popup stays blocked until `G4` passes.
