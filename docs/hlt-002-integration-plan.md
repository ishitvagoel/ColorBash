# HLT-002: Bash highlight integration

Status: `validation` (2026-08-27). ADR 0013. H-1–H-6 and M-1 evidence recorded
(`docs/hlt-comp-review-close-plan.md`). `HLT-003` hostile corpus slices 1–2
recorded (`docs/hlt-003-hostile-gate-plan.md`); p99 `deferred`. Do **not** mark
`HLT-002` complete.

## Goal

Opt-in `MBX_HIGHLIGHT=1` wraps stock `self-insert` in `bash/highlight.bash`.
Plain bytes live in `_MBX_HIGHLIGHT_PLAIN`; Enter restores them before
`accept-line`. Incompatible with `MBX_GHOST=1`.

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx-pty --test highlight -- --nocapture
```
