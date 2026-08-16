# COMP-002 slice: core exact-insertion parity (P-1–P-4)

Status: `validation` (2026-08-16). `COMP-001` is in `validation`. P-1–P-4
and leftover-prep F-1–F-4 evidence is in `bash/completion.bash`,
`tests/bash/modules.bash`, and `crates/pty/tests/completion_harness.rs`.
Do not mark `G4` or `COMP-002` complete. Leftover matrix cases stay out of
scope.

## Why this slice

Immediate next work after `COMP-001`. ADR 0006: stock programmable completion
stays authoritative. Percentile leftovers stay `deferred`. Do not start popup,
ranking, or Git candidates.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Core exact-insertion parity (this plan) | Unblocks `G4` reassessment with file + one `-F` bytes. |
| 2 | Leftover matrix (aliases, Unicode, …) | Blocked on P-1–P-4 landing first. |
| — | Popup / ranking / Git candidates | Blocked on `G4`. |
| — | `PRM-004` / write-ack percentiles | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Adapter-installed shells still insert the same ordinary Bash bytes as stock for
   a unique filename and a filename containing a space.
2. One fixture `-F` wrapped in-process through `_mbx_comp_wrap_backend` inserts
   the same candidate bytes as that `-F` without the wrapper, including
   `compopt -o nospace` (no trailing space) versus default suffix (space before
   the next word).
3. `COMP-002` may move to `validation`. `G4` stays `discovery`.

## Out of scope (hard)

- Aliases, redirections, Unicode, incomplete quotes, `--`, nested commands
- Slow or stateful completion fallthrough
- Completion adapter overhead / 5 ms latency budget
- Popup menu, ranking, descriptions, Git candidates (`COMP-003`+)
- Global wrap of `ls`, `printf`, or default file completion
- Rebinding printable keys or taking Readline ownership
- Auto-executing completed text
- `set -euo pipefail` in sourced modules
- Percentile benches, write-ack, FND-001 SHA refresh
- Marking `G4`, `G3`, `EDT-001`, or `COMP-001` complete
- Committing unless asked

## Method

Keep stock completion authoritative (ADR 0006). File completion stays unwrapped.
Named fixture commands (`mbx_comp_flag`, `mbx_comp_flag_nospace`) install only
when `MBX_COMP_FIXTURES=1`. `_mbx_comp_wrap_existing_f` inspects `complete -p`
before wrapping and skips unknown or non `-F` specs.

Observe insertion through `printf 'GOT:%s|\n' …` so assertions never depend on
history dumps or debug logs (M-023). Reuse `crates/pty/tests/completion_harness.rs`
helpers. Tab is `0x09`. M-019: `wait_all` for output plus next prompt. Do not
wait on CPR/DSR. Use sentinels `MBX_COMP_UNIQUE`, `MBX_COMP_A B`, `--mbx-comp-flag`.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| P-1 | Unique file exact bytes | File `MBX_COMP_UNIQUE`. Type `printf 'GOT:%s|\n' MBX_COMP_U`, Tab, Enter. GOT line matches stock filename completion for that prefix on this host. `ls`/`printf` stay unwrapped. | `validation` — `unique_file_completion_preserves_stock_bytes` (`GOT:MBX_COMP_UNIQUE|`) |
| P-2 | Space-in-name quoting | File `MBX_COMP_A B`. Type `printf 'GOT:%s|\n' MBX_COMP_A`, Tab, Enter. GOT line matches stock quoting/escaping for that name. | `validation` — `spaced_filename_completion_preserves_stock_quoting` (`GOT:MBX_COMP_A B|`) |
| P-3 | Wrapped `-F` + `compopt -o nospace` | `mbx_comp_flag_nospace --mbx-co`, Tab, `X`, Enter. `\nGOT:--mbx-comp-flagX|` then `> `. | `validation` — `wrapped_flag_nospace_concatenates_suffix` |
| P-4 | Wrapped `-F` default suffix | `mbx_comp_flag --mbx-co`, Tab, `X`, Enter. `\nGOT:--mbx-comp-flag X|` then `> `. | `validation` — `wrapped_flag_default_suffix_separates_next_word` |
| F-1 | Default install has no fixtures | Unset `MBX_COMP_FIXTURES`: no `mbx_comp_*` commands and no `complete -F` on those names. Tests set `MBX_COMP_FIXTURES=1`. | `validation` — `default_install_does_not_define_fixtures` |
| F-2 | Inspect-before-wrap | `_mbx_comp_wrap_existing_f` wraps a caller-defined `-F`, skips absent specs, skips `-W`. | `validation` — `tests/bash/modules.bash` |
| F-4 | H-2 asserts POINT/CWORD | PTY dump is `MBX_COMP:mbx_comp_probe mbx_co:21:1:mbx_comp_candidate` (this host). Module test asserts `COMP_WORDS` count. | `validation` — `probe_snapshot_captures_comp_state` |

## Remaining after this slice

Leftover `G4` matrix: aliases, redirections, Unicode, incomplete quotes, `--`,
nested commands, slow/stateful fallthrough, and the provisional 5 ms adapter
overhead budget. Popup stays blocked until `G4` passes.
