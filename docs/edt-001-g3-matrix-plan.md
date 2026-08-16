# EDT-001 slice: G3 matrix after E-1–E-4 (M-1–M-4)

Status: `validation` (2026-08-16). M-1–M-4 PTY evidence in
`crates/pty/tests/editor_bind_x.rs`. Exact-byte / quoting / multiline insert
remains. Do not mark `G3` or `EDT-001` complete unless every G3 exit bullet has
evidence.

## Why this slice

Immediate next work after E-1–E-4. `COMP-001` may start later; do not mix it
here. Percentile leftovers stay `deferred`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | G3 matrix M-1–M-4 (this plan) | Remaining named G3 bullets that this host can evidence. |
| 2 | Exact-byte / quoting / multiline insert | Next EDT leftover after M-1–M-4. |
| 3 | `COMP-001` harness | Ready, but G3 is the active workstream. |
| — | Ghost / popup / highlighting / Ctrl+R UI | Blocked on `G3`. |
| — | `PRM-004` / write-ack percentiles | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. The existing insert action works in emacs (already) and **vi-insert**.
2. Bracketed paste does not execute inserted or pasted text until Enter.
3. Resize after insert leaves the line and next prompt usable.
4. Ctrl+Z after a running command in an insert-capable session returns a
   usable prompt; insert still works afterward.
5. `G3` stays `discovery`. `EDT-001` stays `validation` unless every G3
   bullet is evidenced (it will not be after this slice).

## Out of scope (hard)

- Rebinding printable keys or taking Readline ownership (ADR 0003)
- Auto-executing inserted text
- Ghost, completion popup, syntax highlighting, `COMP-001`
- Exact-byte / quoting / multiline matrix (next leftover)
- Percentile benches, write-ack, FND-001 SHA refresh
- `set -euo pipefail` in sourced modules
- Marking `G3` complete
- Committing unless asked

## Method

Keep Readline responsible for editing and redisplay. Default emacs install
already uses `bind -x`. For vi, also install on the **vi-insert** keymap
(`bind -m vi-insert -x`) so `set -o vi` still inserts without execute.
Inspect occupied chords per keymap; refuse unless `MBX_EDITOR_OVERRIDE=1`.
Do not bind vi-command unless a test proves the default chord is free and
needed; prefer vi-insert only.

Reuse `crates/pty/tests/editor_bind_x.rs` helpers. M-019: `wait_all` for
output plus next prompt. Do not wait on CPR/DSR. Assert non-execution with
`\nMBX_EDT:ok`, not the echoed line.

Bracketed paste: send `ESC[200~` … `ESC[201~` around ordinary bytes. Do not
treat paste as an execute path.

Ctrl+Z: start a foreground `sleep` (same pattern as `foundation.rs`), send
Ctrl+Z, wait for the next prompt, then prove insert still works. Do not
require `stty -g` here; foundation already covers that.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| M-1 | vi-insert insert without execute | `set -o vi`, enter insert mode if needed, trigger the configured chord, token is not executed until Enter; `\nMBX_EDT:ok` then `> ` | `complete` |
| M-2 | Bracketed paste does not execute | Paste a prefix (or the token) with bracketed-paste delimiters; optional insert trigger; no `\nMBX_EDT:ok` until Enter; then output plus next prompt | `complete` |
| M-3 | Resize after insert | Trigger insert, resize (e.g. 16x64), Enter still runs the token; next prompt usable | `complete` |
| M-4 | Ctrl+Z then insert still works | After a usable prompt, start a sleep job, Ctrl+Z, next `> ` usable, then insert+Enter yields `\nMBX_EDT:ok` plus `> ` | `complete` |

## Remaining after this slice

Exact-byte / quoting / multiline insert is complete (`docs/edt-001-exact-bytes-plan.md` B-1–B-4). B-5 redraw note recorded. `G3` is in `validation`; `COMP-001` may start next.
