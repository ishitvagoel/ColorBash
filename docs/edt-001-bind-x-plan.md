# EDT-001 slice: non-destructive bind -x insertion prototype (E-1–E-4)

Status: `validation` (2026-08-16). E-1–E-4 PTY evidence in
`crates/pty/tests/editor_bind_x.rs`; implementation in `bash/editor.bash`. Do not
mark `G3` or `EDT-001` complete unless the roadmap G3 exit criteria are fully met.

## Why this slice

`G2` is complete. Unmet percentile leftovers are `deferred`
(`docs/latency-budget-deferral.md`). Immediate product work is `EDT-001` → `G3`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | bind -x insertion prototype (this plan) | Unblocks G3 discovery with PTY evidence. ADR 0003: augment Readline; do not take editor ownership. |
| 2 | Full G3 matrix (emacs/vi, paste, resize, Ctrl+C/Z restoration) | Later EDT slice after E-1–E-4. |
| — | `PRM-004` / write-ack percentiles | `deferred`. Do not spend this packet on timing. |
| — | Ghost / completion popup / highlighting | Blocked on `G3`. |
| — | `HRD-001` macOS | Needs a macOS host; not this slice. |

## Goal

1. One configurable `bind -x` action inserts ordinary Bash text into
   `READLINE_LINE` at `READLINE_POINT` and does **not** execute it.
2. Existing unknown bindings are not overwritten unless the user opts in.
3. PTY evidence: typed trigger inserts bytes; the line is not run until the
   user presses Enter; next prompt stays usable (M-019-safe waits).
4. `EDT-001` may move to `validation` after E-1–E-4. `G3` stays `discovery`
   until the remaining G3 bullets have evidence.

## Out of scope (hard)

- Rebinding printable keys or taking Readline ownership (ADR 0003 stop condition)
- Auto-executing inserted text
- Ghost rendering, completion popup, syntax highlighting
- Percentile benches, write-ack chase, FND-001 SHA refresh
- `set -euo pipefail` in sourced modules
- Marking `G3` complete on this slice alone
- Committing unless asked

## Method

Keep Readline responsible for editing and redisplay. Install a **named** function
and bind it to a **configurable** unused chord (default a Ctrl+X prefix or an
explicit test-only sequence). Inspect `bind -q` / current bindings before
installing; skip or refuse if the chord is already taken unless an explicit
override is set.

The function may only:

- read `READLINE_LINE` / `READLINE_POINT`;
- insert a fixed ordinary-Bash test token (no command text from history);
- update `READLINE_POINT` to after the insertion;
- return.

Reuse `crates/pty` helpers. Do not wait on CPR/DSR.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| E-1 | Insert without execute | PTY: trigger inserts a sentinel token into the line; the token is not executed until Enter; output of the sentinel command appears only after Enter | `complete` |
| E-2 | Unknown binding preserved | If the chosen chord is already bound, MBX does not overwrite it unless an explicit override env/flag is set; test both “occupied → refuse” and “free → install” | `complete` |
| E-3 | Empty / no-op safety | Trigger on an empty line still only inserts; does not submit the line | `complete` |
| E-4 | Usable next prompt | After insert+Enter (or insert+Ctrl+C cancel of the line), the next `> ` is usable; `stty` not required in this slice if foundation tests already cover it | `complete` |

## Remaining after this slice

Exact bytes, cursor position, suffixes, quoting, and multiline input.
Redraw-without-rebinding-printables write-up. G3 matrix M-1–M-4 is complete
(`docs/edt-001-g3-matrix-plan.md`); `G3` stays `discovery` until those bullets
and the redraw assessment are evidenced.
