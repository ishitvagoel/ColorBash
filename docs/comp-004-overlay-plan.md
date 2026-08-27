# COMP-004 overlay slice

Status: `complete` (2026-08-27). ADR 0013. Tab stays stock.

## Goal

Opt-in `MBX_COMP_OVERLAY=1` snapshots ranked completion rows. `\C-x\C-o` toggles a
bounded candidate list below the prompt; `\C-xn`/`\C-xp` move selection while
visible; `\C-x\C-a` accepts; `\C-g` dismisses.

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx-pty --test completion_harness overlay_lists ranked_accept_works_with_overlay_env -- --nocapture
```
