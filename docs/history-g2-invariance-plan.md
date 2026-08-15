# HIST-007 slice: `.bash_history` invariance and recorder admission parity

Status: `complete` for the invariance/admission-parity PTY slice (2026-08-15).
100k-row budgets, contention, hostile SQL/control end-to-end, and permission
checks remain later `G2` slices.

## Goal

Prove two contracts with genuine PTY tests:

1. **Invariance.** Enabling, disabling, clearing, or deleting the sidecar causes
   no extra `.bash_history` changes beyond normal Bash behavior.
2. **Admission parity.** With `MBX_HISTORY=1`, the sidecar records only
   Bash-admitted entries from the `HIST-002` matrix — not executed-but-dropped
   commands.

The sidecar is an admission event log, not a live mirror of `HISTFILE`.
`erasedups` and `history -d` change Bash’s list without deleting earlier sidecar
rows; tests must not require those lists to be equal.

## Out of scope

- 100k-row corpus, write/query percentiles, contention, WAL crash, foreign-user
  open
- Fuzzy ranking, repository context, editor UI, default-on capture
- MBX1/MBX2 framing changes
- Product code unless a test finds a real `HISTFILE` write or a wrong record

## Method

Extend the recording PTY harness. Dump `HISTFILE` the same way as
`history_admission.rs`: source a script that runs `history -a`, wait for a marker
that never appears in typed-command echo plus the next prompt (`wait_all`,
`M-019`), then read the file from disk. Poll sidecar count while the helper is
still alive before `exit`.

Paired sessions use the same typed commands and dump method. One leaves
`MBX_HISTORY` unset (or `=0`); the other sets `MBX_HISTORY=1`. Compare
`HISTFILE` bytes. Drive `clear`/`delete` as out-of-band `mbx` subprocesses so
those controls are not themselves typed history events.

## Test cases

| ID | Case | Assert |
| --- | --- | --- |
| INV-1 | Enable vs default-off, same `echo a`; `echo b` | `HISTFILE` bytes equal; enabled store has 2 rows; disabled creates no store |
| INV-2 | `MBX_HISTORY=0` vs unset | `HISTFILE`s equal; neither creates a store |
| INV-3 | External `mbx history clear` after two records | `HISTFILE` unchanged; count is 0; prompt still works |
| INV-4 | External `mbx history delete` | `HISTFILE` unchanged; sqlite/WAL/SHM unlinked; later prompt still usable |
| INV-5 | Seeded prior `echo prior` then one new command | `echo prior` appears once on both sides |
| INV-6 | Exit flush, no mid-session dump | `HISTFILE`s match after clean `exit` |
| ADM-1 | `HISTCONTROL=ignorespace` | Sidecar has only the unspaced command |
| ADM-1b | No `HISTCONTROL`, leading-space command | Sidecar preserves the leading space (`M-022`) |
| ADM-2 | `HISTCONTROL=ignoredups` | One `echo dup` plus `keep` |
| ADM-3 | `HISTIGNORE='rm *'` | `rm` executes; sidecar has only `keep` |
| ADM-4 | History off | Sidecar has `visible`, not `hidden`, not `set -o history` |
| ADM-5 | `history -s` | Injected marker is stored and not executed |
| ADM-6 | Multiline fold | One sidecar row `echo one two` |
| ADM-7 | Empty Enter after a recorded command | No extra sidecar row |
| ADM-8 | `MBX_HISTORY_EXCLUDE='git *'` | `HISTFILE` still has `git status`; sidecar has only `keep` |

## Edge cases

- Do not compare sidecar contents to the current `HISTFILE` list.
- The dump script is itself a history event; both paired sessions must dump the
  same way, or compare only after exit flush.
- Out-of-band clear/delete: typing `mbx history clear` would be admitted by Bash.
- After `delete`, do not run `mbx history count` before asserting files are gone;
  opening the store recreates it.
- `wait_all` for output plus prompt; never re-wait for `> ` after matching output.
- `PS2` must be distinct from the MBX `> ` prompt for the continuation case.

## Implementation checklist

1. Confirm production code never opens `HISTFILE` for write.
2. Share PTY helpers (dump-from-disk, paired spawn, out-of-band controls).
3. Implement INV-1–INV-6 and ADM-1–ADM-8 (plus ADM-1b).
4. Keep recording-test 1.0s transport timeouts; do not treat them as `G2` budgets.
5. Run the new PTY tests, then `bash tests/run.bash`.
6. Update `HIST-007` evidence notes only. Do not mark `G2` complete.

## Follow-on `G2` slices

Hostile SQL/control inertness, permission checks beyond mode bits, the seeded
100k corpus, query/write percentiles, and the contention cases in
`docs/benchmarks/history-budgets.md`.
