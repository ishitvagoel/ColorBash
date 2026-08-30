# COMP-004 overlay slice

Status: `validation` (2026-08-27; reviewed 2026-08-29). ADR 0013. Tab stays
stock. O-1–O-5 evidence recorded (`docs/hlt-comp-review-close-plan.md`).
Dismiss chord is `\C-xj`. Do **not** mark `COMP-004` complete.

**`M-065` (open, confirmed 2026-08-29):** the overlay's `\e7`/`\e8`
(DECSC/DECRC) save-and-restore is an absolute screen position. Drawing
enough rows to scroll the terminal invalidates it; the subsequent `\e[J`
then erases the prompt and prior output from the wrong, stale origin.
Reproduced with a real PTY and a purpose-built VT screen model
(`crates/pty/src/screen.rs`); the failing case is
`crates/pty/tests/overlay_screen.rs`
(`overlay_near_the_bottom_of_a_short_terminal_leaves_the_prompt_intact`,
kept `#[ignore]` so it documents the defect without failing the canonical
suite — run with `-- --ignored` to reproduce). A `SIGWINCH` while the
overlay is visible is unaffected (`resize_while_overlay_is_visible_leaves_a_usable_prompt`
in the same file); the defect is specifically scroll-during-draw. A correct
fix needs either a bounded DSR (`\e[6n`) cursor-row query — which this
codebase does not have — or a different rendering strategy; a naive
relative-cursor-up substitute fixes the row but not the column, and a
subsequent blind `\e[J` from the wrong column would then corrupt the prompt
line instead of the overlay. See `MISTAKES.md` M-065. `COMP-004` cannot move
to `complete` until this is resolved by its own reviewed slice or an
accepted descope.

## Goal

Opt-in `MBX_COMP_OVERLAY=1` snapshots ranked completion rows. `\C-x\C-o` toggles a
bounded candidate list below the prompt; `\C-xn`/`\C-xp` move selection while
visible; `\C-x\C-a` accepts; `\C-xj` dismisses (stock `\C-g` `abort` stays).

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx-pty --test completion_harness overlay_lists ranked_accept_works_with_overlay_env -- --nocapture
```
