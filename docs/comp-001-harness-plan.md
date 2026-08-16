# COMP-001 slice: non-popup stock-completion adapter harness (H-1–H-4)

Status: `complete` (2026-08-16). `G4` is `complete`. H-1–H-4 evidence is in `bash/completion.bash`, `tests/bash/modules.bash`, and
`crates/pty/tests/completion_harness.rs`. Do not mark `G4` or `COMP-001`
complete. `COMP-002` owns file and `-F` exact-parity evidence.

## Why this slice

Immediate next work. ADR 0006: stock programmable completion stays
authoritative. Percentile leftovers stay `deferred`. Do not start popup,
ghost, or highlighting.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Completion adapter harness (this plan) | Unblocks `COMP-002` / `G4` with a snapshot + fallthrough seam. |
| 2 | `COMP-002` file + one `-F` parity matrix | Blocked on this harness. |
| — | Popup / ranking / Git candidates | Blocked on `G3` and `G4`. |
| — | `PRM-004` / write-ack percentiles | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. A sourced `bash/completion.bash` module can wrap one known `-F` spec and
   snapshot `COMP_LINE`, `COMP_POINT`, `COMP_WORDS`, `COMP_CWORD`, `COMP_TYPE`,
   and `COMP_KEY` without changing insertion bytes.
2. Unknown or unsupported `complete` specs fall through; the line is not
   mutated by the adapter.
3. PTY: Tab on a unique filename still inserts the same ordinary Bash text
   as stock (harness smoke, not the `COMP-002` matrix).
4. `COMP-001` may move to `validation`. `G4` stays `discovery`. Do not mark
   `G4` complete.

## Out of scope (hard)

- Popup menu, ranking, descriptions, Git candidates (`COMP-003`+)
- Full `COMP-002` matrix (aliases, redirections, Unicode, incomplete quotes,
  `--`, nested commands, latency budget)
- Rebinding printable keys or taking Readline ownership
- Auto-executing completed text
- `set -euo pipefail` in sourced modules
- Percentile benches, write-ack, FND-001 SHA refresh
- Marking `G4` or `G3` complete
- Committing unless asked

## Method

Keep stock completion authoritative (ADR 0006). Install a **named** wrapper
only for an explicit test command (for example `mbx_comp_probe`) registered
with `complete -F`. The wrapper:

- copies `COMP_*` into `_MBX_COMP_*` snapshot variables (or a single
  newline-safe dump that never includes history command text);
- invokes the original `-F` function in-process (do not subprocess);
- leaves `COMPREPLY` and `compopt` to that function;
- returns.

Default file completion must remain stock. Do not replace `-o default` or
`complete -o filenames` globally. Inspect `complete -p NAME` before wrapping;
skip unknown specs.

Reuse `crates/pty` helpers. Tab is `0x09`. M-019: `wait_all` for output plus
next prompt. Do not wait on CPR/DSR. Do not log command text (M-023); use
sentinel filenames such as `MBX_COMP_UNIQUE`.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| H-1 | Module sourced, no `set -euo pipefail` | `bash/completion.bash` is sourced from `init.bash`. Module contract rejects `set -euo pipefail` and `MBX_DBG`. Install is idempotent. | `validation` — `tests/bash/modules.bash` |
| H-2 | Snapshot a known `-F` | Register `mbx_comp_probe` with a test `-F` that appends one candidate. After Tab (or a sourced trigger), `_MBX_COMP_LINE` / `_MBX_COMP_POINT` / `_MBX_COMP_CWORD` match the typed prefix. `COMPREPLY` still contains the candidate. | `validation` — `probe_snapshot_captures_comp_state` |
| H-3 | Unknown spec falls through | A command with no MBX wrap (`complete -p` absent or a stock spec) is not replaced. Adapter does not bind it. Line after Tab matches stock behavior for that command (or no extra wrapper in `complete -p`). | `validation` — `stock_ls_completion_is_not_wrapped` |
| H-4 | Unique file Tab smoke | Temp dir with file `MBX_COMP_UNIQUE`. Type `ls MBX_COMP_U` then Tab. Inserted buffer becomes `ls MBX_COMP_UNIQUE` (or with trailing space if stock adds it). Next prompt usable after Enter. Do not invent extra filename cases. | `validation` — `unique_filename_tab_completes_like_stock` |

## Remaining after this slice

`COMP-002` P-1–P-4 and leftover-prep F-1–F-4 have landed. Next is the leftover
`G4` matrix (aliases, redirections, Unicode, incomplete quotes, `--`, nested
commands, slow/stateful fallthrough, adapter latency). Popup stays blocked.
