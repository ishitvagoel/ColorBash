# SRCH-001 leftover: explicit history-search `bind -x` (S-1–S-6)

Status: `validation` for the explicit insert action (2026-08-16). ADR 0009
unblocks Phase 8 from the continuous-decoration leftover. This packet adds a
configurable chord that queries the opt-in sidecar and replaces `READLINE_LINE`
with one match without executing it. It does **not** add a metadata result
view, match cycling, cancel restoration, or steal stock `\C-r`.

## Why this slice

Ghost, highlighting, and a GUI overlay still need an after-every-key hook.
Enhanced Ctrl+R does not: it is an explicit `bind -x` action, the same class as
ranked-accept. `G2` and `G3` are complete. The CLI already prints one command
text per search line (`crates/cli/src/app.rs`).

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Explicit search insert (this plan) | Named Phase 8 start; ADR 0009; `bind -x` pattern exists. |
| 2 | Result view / match cycling | `SRCH-001` leftover; needs a list UX without an overlay. |
| — | Cancel restoration (`SRCH-002`) | Depends on a search mode that can restore the original buffer. |
| — | Metadata filters / 100k interactive (`SRCH-003`) | After `SRCH-002`; repo filters also need `HIST-010`. |
| — | Steal stock `\C-r` | Rejected by ADR 0009 unless the user opts in. |
| — | Ghost / highlighting / overlay | Still blocked on after-every-key decoration. |
| — | `HIST-010` / `GIT-003` | Separate leftover; do not mix. |
| — | Percentile benches / `HRD-001` | `deferred` / needs a macOS host. |

## Goal

1. Install an optional `bind -x` chord (default `\C-x\C-r`, overridable via
   `MBX_SEARCH_KEYSEQ`) in emacs and vi-insert keymaps. Reuse the occupied-keyseq
   skip pattern from `bash/editor.bash` (`MBX_SEARCH_OVERRIDE=1` to overwrite).
   Do not rebind Tab, printables, or stock `\C-r`.
2. On the chord, if `MBX_HISTORY` is not exactly `1`, leave `READLINE_LINE`
   unchanged. If the helper is missing, times out, or fails, leave the line
   unchanged. Never execute inserted text.
3. Query is `READLINE_LINE`. Empty → `mbx history search recent --limit 1`.
   Non-empty → `search prefix QUERY --limit 1`, then `search fuzzy QUERY --limit 1`
   when prefix returns no line. Take the first printed command text, replace the
   entire line, set `READLINE_POINT` to the end (do not splice; M-039 analog).
4. Bound the helper with `MBX_SEARCH_TIMEOUT` (default `MBX_HISTORY_TIMEOUT` /
   0.10 s) using the existing engine child read/kill primitives. Redirect helper
   stderr. Do not log command text (`M-023`).
5. `SRCH-001` stays `validation` (result view unbuilt). Do not mark `SRCH-001`,
   `SRCH-002`, `SRCH-003`, `COMP-004`, or `COMP-005` complete. Do not start
   ghost, overlay, or highlighting.

## Out of scope (hard)

- Rebinding `\C-r`, Tab, arrows, or printable keys
- Overlay, candidate list rendering, age/cwd/status columns
- Match cycling or a persistent search mode
- Cancel restoration of the pre-search buffer (`SRCH-002`)
- Repository / failed-command filters (`HIST-010` / `SRCH-003`)
- Ghost, highlighting, completion popup
- Changing MBX1 framing
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Percentile benches
- Marking `SRCH-001` complete
- Committing unless asked
- Widening into continuous decoration

## Method

Read `bash/editor.bash`, `bash/completion.bash` ranked-accept, `bash/history.bash`,
ADR 0003, ADR 0005, ADR 0009, and `crates/pty/tests/editor_bind_x.rs`. Keep Bash
as the only executor.

Add `bash/search.bash` and source it from `bash/init.bash`. Install after
`MBX_BIN` is resolved. Skip `bind` when `$-` lacks `i`. Keep install idempotent.

## Asserts

| ID | Evidence |
| --- | --- |
| S-1 | History-enabled PTY: record `printf 'MBX_SRCH:alpha\n'` then `printf 'MBX_SRCH:beta\n'`; type `printf 'MBX_SRCH:a`; `\C-x\C-r` does not execute; Enter prints `MBX_SRCH:alpha` |
| S-2 | Same session empty line + chord inserts the newest row (`printf 'MBX_SRCH:beta\n'`) and Enter prints `MBX_SRCH:beta` |
| S-3 | Occupied `\C-x\C-r` is not overwritten; `_MBX_SEARCH_BOUND=0` |
| S-4 | `MBX_HISTORY` unset: chord leaves the typed line unchanged (module stub would insert if called) |
| S-5 | Substring that is not a prefix uses fuzzy fallback (`needle` → `echo MBX_SRCH:zzz-needle`) |
| S-6 | Missing helper / non-executable `MBX_BIN`: no-op |

## Docs

- `docs/adr/0009-explicit-history-search-bind-x.md`
- `docs/roadmap.md` Phase 8 / `SRCH-001` / immediate next
- `docs/architecture.md`, `docs/bash-compatibility.md`, `docs/ux-spec.md`
- `README.md` tryable row and manual test

## Stop

Do not start the result-view leftover, `SRCH-002`, ghost, or overlay. Do not
mark `SRCH-001` complete.
