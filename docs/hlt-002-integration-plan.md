# HLT-002: Bash highlight integration

Status: `complete` (2026-08-31). ADR 0015. `READLINE_LINE` stays permanently
plain; the helper's styled copy paints on one reserved row below the prompt
(M-065 IND/DECSC). H-1–H-6 and M-1 evidence recorded
(`docs/hlt-comp-review-close-plan.md`). `HLT-003` hostile corpus slices 1–2
recorded (`docs/hlt-003-hostile-gate-plan.md`); p99 `deferred`.

## Goal

Opt-in `MBX_HIGHLIGHT=1` wraps stock `self-insert` in `bash/highlight.bash`.
The edit buffer stays ordinary Bash text. Styled bytes are a tty preview,
never assigned to `READLINE_LINE`. Incompatible with `MBX_GHOST=1`. Mutually
exclusive with a visible completion overlay.

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx-pty --test highlight -- --nocapture
```
