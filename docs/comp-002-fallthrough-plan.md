# COMP-002 slice: slow/stateful wrap fallthrough (S-1–S-4)

Status: `ready` (2026-08-16). N-1–N-2 are in `validation`. This packet proves
G4's remaining wrap-seam cases: unsupported specs stay unwrapped, a slow `-F`
still inserts its own bytes, a stateful `-F` sees live shell state, and an
empty `COMPREPLY` does not invent a candidate. Do not mark `G4` or `COMP-002`
complete.

The 5 ms adapter overhead budget stays `deferred`. Do not start it here.

## Why this slice

Immediate next work after N-1–N-2. ADR 0006 / G4 still name unsupported, slow,
and stateful completion through safe fallthrough. This is the
`_mbx_comp_wrap_existing_f` seam, not unwrapped file completion.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Slow/stateful wrap fallthrough (this plan) | Named G4 wrap cases still without PTY bytes. |
| 2 | Adapter 5 ms budget | `deferred` with other percentiles. |
| — | Popup / ranking / Git candidates | Blocked on `G4`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. `_mbx_comp_wrap_existing_f` still skips a non-`-F` spec (`-W`). Tab inserts
   that word-list candidate; `complete -p` is unchanged.
2. A wrapped `-F` that sleeps `0.2s` then returns one unique candidate still
   inserts those bytes. The next prompt is usable. Do not add a timeout.
3. A wrapped `-F` that reads a shell variable set **after** wrap still inserts
   that live value. Do not subprocess the backend.
4. A wrapped `-F` that sets `COMPREPLY=()` inserts nothing. The typed prefix
   is unchanged.
5. `ls` and `printf` stay unwrapped. Default install still defines no
   `mbx_comp_*`. Do not add these four names to `_mbx_completion_install`.
6. `COMP-002` stays `validation`. `G4` stays `discovery`.

## Out of scope (hard)

- Completion adapter overhead / 5 ms latency budget / percentile benches
- Timeouts, deadlines, or aborting a slow `-F`
- Moving a completion function into a subprocess
- Popup, ranking, descriptions, Git candidates (`COMP-003`+)
- Wrapping `ls`, `printf`, or default file completion
- Rebinding printable keys or taking Readline ownership
- Auto-executing completed text
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Pipelines, `` `...` ``, a second nested form, or new file-completion cases
- Marking `G4`, `G3`, `EDT-001`, `COMP-001`, or `COMP-002` complete
- Rewriting this plan's cases or adding extra commands
- Committing unless asked

## Method

Keep stock completion authoritative (ADR 0006). Call
`_mbx_comp_wrap_existing_f` only on the four test commands in this plan.
Define those commands in `rc_prelude` (functions + `complete`); wrap them
**after** `init.bash` (after the first prompt). Use
`spawn_mbx_shell` **without** `MBX_COMP_FIXTURES` (M-037).

Do not add `mbx_comp_words`, `mbx_comp_slow`, `mbx_comp_state`, or
`mbx_comp_empty` to `bash/completion.bash`.

Tab is `0x09`. M-019: `wait_all` for the GOT line plus `> `. Do not wait on
CPR/DSR. Do not log command text (M-023). Observe through
`printf 'GOT:%s|\n'` in the command function, same as P-3 / P-4.

If a measured GOT line differs from the expected column, keep the host's
stock bytes and record them in this plan's Status cell. Do not normalize
quoting.

Each case uses one isolated `TempHome`. No sentinel files are required.

## Test cases

| ID | Case | Setup (`rc_prelude`, then wrap after prompt) | Type, then Tab, then Enter | Expected GOT | Status |
| --- | --- | --- | --- | --- | --- |
| S-1 | Unsupported `-W` skip | `mbx_comp_words() { printf 'GOT:%s\|\n' "$*"; }` and `complete -W 'mbx_word_alpha' mbx_comp_words`. After prompt: `_mbx_comp_wrap_existing_f mbx_comp_words` (must fail / skip). | `mbx_comp_words mbx_w` Tab Enter | `\nGOT:mbx_word_alpha\|` then `> `. `complete -p mbx_comp_words` still contains `-W` and not `_mbx_comp_existing_adapter`. | pending |
| S-2 | Slow wrapped `-F` | `mbx_comp_slow() { printf 'GOT:%s\|\n' "$*"; }` and `_mbx_comp_slow_backend` that `sleep 0.2` then `COMPREPLY=(--mbx-comp-slow)`. `complete -F _mbx_comp_slow_backend mbx_comp_slow`. After prompt: `_mbx_comp_wrap_existing_f mbx_comp_slow`. | `mbx_comp_slow --mbx-sl` Tab Enter | `\nGOT:--mbx-comp-slow\|` then `> `. Do not hang. | pending |
| S-3 | Stateful wrapped `-F` | `mbx_comp_state() { printf 'GOT:%s\|\n' "$*"; }` and `_mbx_comp_state_backend` that does `COMPREPLY=("${MBX_COMP_STATE_TOKEN:-missing}")`. `complete -F _mbx_comp_state_backend mbx_comp_state`. After prompt: wrap, then `MBX_COMP_STATE_TOKEN=live-alpha`. | `mbx_comp_state liv` Tab Enter | `\nGOT:live-alpha\|` then `> `. Must not print `missing`. | pending |
| S-4 | Empty `COMPREPLY` | `mbx_comp_empty() { printf 'GOT:%s\|\n' "$*"; }` and `_mbx_comp_empty_backend` that does `COMPREPLY=()`. `complete -F _mbx_comp_empty_backend mbx_comp_empty` with **no** `-o default` / `-o bashdefault`. After prompt: wrap. | `mbx_comp_empty nosuch` Tab Enter | `\nGOT:nosuch\|` then `> `. Tab inserts nothing. | pending |

### S-1 notes

This is the PTY complement of the existing F-2 module skip. Do not replace the
`-W` spec. Do not invent a second unsupported form (`-C`, `-A`, `-G` stay
module-only). After wrap, `complete -p` must match the pre-wrap spec.

### S-2 notes

`sleep 0.2` is enough to be slower than an instant `-F` and short enough for
the existing PTY deadline. Do not add a timeout, deadline, or percentile
measurement. Do not use `sleep 10`. The wrap must keep the backend in-process.

### S-3 notes

Set `MBX_COMP_STATE_TOKEN` **after** wrap so a wrap-time snapshot cannot
accidentally pass. `liv` is a unique prefix of `live-alpha`. If the adapter
subprocessed the backend, the variable would be unset and GOT would be
`missing`. That is a defect.

### S-4 notes

Empty `COMPREPLY` is a failed/no-match `-F`. The adapter must not invent a
filename or flag. Do not add `-o default`. The typed word `nosuch` is the
entire argument.

## Module contract (no new fixture names)

Keep the existing F-1 / F-2 / probe / flag / L-1–L-4 / N-1–N-2 asserts. Add
only:

- `complete -p ls` and `complete -p printf` still contain no `_mbx_comp`
- default install still defines no `mbx_comp_*`
- `_mbx_comp_wrap_existing_f` still skips `-W` (already in `tests/bash/modules.bash`)

Do not add slow/state/empty logic to `bash/completion.bash` unless a test
proves the adapter mutates these lines or drops live state today. The expected
outcome is **no product change** beyond tests and this plan's status column.
Do not add a timeout.

## Remaining after this slice

The provisional 5 ms adapter overhead budget (`deferred`). Then a `G4`
decision. Popup stays blocked until `G4` passes.
