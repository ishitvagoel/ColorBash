# EDT-001 slice: exact-byte / quoting / multiline insert (B-1–B-4)

Status: `validation` (2026-08-16). B-1–B-4 PTY evidence in
`crates/pty/tests/editor_bind_x.rs`. B-5 redraw note below. `G3` may move to
`validation` but must not be marked `complete` (continuous decoration unproven).

## Why this slice

Immediate next work after M-1–M-4. `COMP-001` may start later; do not mix it
here. Percentile leftovers stay `deferred`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Exact-byte / quoting / multiline (this plan) | Last named G3 insert-semantics leftover on this host. |
| 2 | `COMP-001` harness | Ready, but G3 is the active workstream. |
| — | Ghost / popup / highlighting / Ctrl+R UI | Blocked on `G3`. |
| — | `PRM-004` / write-ack percentiles | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. Mid-line insert preserves prefix and suffix bytes.
2. After insert, typed characters land after the token (`READLINE_POINT`).
3. Insert inside single quotes is data; the token is not executed.
4. Insert on a `PS2` continuation preserves exact bytes; next prompt usable.
5. Short redraw note: insert-time Readline redraw works without rebinding
   printables; continuous decoration remains unproven.
6. `EDT-001` stays `validation`. `G3` moves to `validation` with the B-5 redraw
   note; do not mark `G3` complete.

## Out of scope (hard)

- Rebinding printable keys or taking Readline ownership (ADR 0003)
- Auto-executing inserted text
- Ghost, completion popup, syntax highlighting, `COMP-001`
- Percentile benches, write-ack, FND-001 SHA refresh
- `set -euo pipefail` in sourced modules
- Marking `G3` complete
- Committing unless asked

## Method

Reuse `_mbx_editor_insert_token` in `bash/editor.bash`. For exact-byte cases
set `MBX_EDITOR_INSERT_TOKEN=MBX_EDT_TOKEN` (plain identifier, not a command).
Observe bytes through `printf 'GOT:%s\n' '…'` so assertions never depend on
history dumps or command-text logs (M-023).

Reuse `crates/pty/tests/editor_bind_x.rs` helpers. M-019: `wait_all` for
output plus next prompt. Do not wait on CPR/DSR. Assert non-execution with
`\nMBX_EDT:ok`, not the echoed line.

Cursor motion: emacs `Ctrl+B` (`0x02`) only. Do not rebind printables.

Multiline: set `PS2=CONT> ` like `crates/pty/tests/multiline_width.rs`. Open
a quote, Enter, wait for `CONT> `, then insert.

Redraw note: add a short paragraph to this plan (or
`docs/research/bash-readline-investigation.md`) after the tests pass. Do not
write a new ADR unless the evidence shows printable-key rebinding is required
(stop condition).

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| B-1 | Mid-line prefix and suffix | Token `MBX_EDT_TOKEN`. Type `printf 'GOT:%s\n' 'XX'`, send `Ctrl+B` three times (cursor on first `X`), trigger insert, Enter. Output `\nGOT:MBX_EDT_TOKENXX` then `> `. | `complete` |
| B-2 | Cursor after insert | Token `MBX_EDT_TOKEN`. Type `printf 'GOT:%s\n' '`, trigger insert, type `YY'`, Enter. Output `\nGOT:MBX_EDT_TOKENYY` then `> `. | `complete` |
| B-3 | Quoted insert is not executed | Default token. Type `printf 'GOT:%s\n' '`, trigger insert, type `'`, Enter. Output contains `GOT:` plus the token text; no `\nMBX_EDT:ok` command output; then `> `. | `complete` |
| B-4 | Multiline / PS2 insert | Token `MBX_EDT_TOKEN`. `PS2=CONT> `. Type `printf 'GOT:%s\n' '` then Enter, wait `CONT> `, trigger insert, type `'`, Enter. Output contains `GOT:` and `MBX_EDT_TOKEN`; then `> `. | `complete` |

## B-5 — Insert-time redraw assessment

PTY evidence (E-1–E-4, M-1–M-4, B-1–B-4) shows that explicit `bind -x`
insertions redraw through Readline without rebinding printable keys. Mid-line,
quoted, and `PS2` continuation buffers keep exact bytes; Readline redisplay
after insert does not require MBX to own cursor motion or terminal redraw.

Continuous syntax decoration, ghost text, and popup completion still lack a
supported after-every-key hook. Those features remain blocked until a separate
experiment proves a safe redraw strategy or revisits ADR 0003. This note does
not claim ghost or highlighting feasibility.

## Remaining after this slice

The `G3` decision is specified in `docs/g3-decision-plan.md`. `G3` stays
`validation` until continuous-decoration strategy is decided. Do not start
ghost, popup, or `COMP-003`.
