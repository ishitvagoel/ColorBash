# HLT-002: Bash highlight integration

Status: `complete` (2026-08-27). ADR 0013.

## Goal

Opt-in `MBX_HIGHLIGHT=1` wraps stock `self-insert` in `bash/highlight.bash`.
Plain bytes live in `_MBX_HIGHLIGHT_PLAIN`; Enter restores them before
`accept-line`. Incompatible with `MBX_GHOST=1`.

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx-pty --test highlight -- --nocapture
```
