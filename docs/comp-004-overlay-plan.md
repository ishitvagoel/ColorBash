# COMP-004 overlay slice

Status: `validation` (2026-08-27; reviewed 2026-08-29). ADR 0013. Tab stays
stock. O-1–O-5 evidence recorded (`docs/hlt-comp-review-close-plan.md`).
Dismiss chord is `\C-xj`. Do **not** mark `COMP-004` complete.

**`M-065` (fixed 2026-08-30; confirmed 2026-08-29):** the overlay's `\e7`/`\e8`
(DECSC/DECRC) save-and-restore is an absolute screen position. Drawing enough
rows to scroll the terminal invalidated it, and the subsequent `\e[J` then
erased from the wrong, stale origin — leaving the overlay's own rows stranded
on screen and destroying the scrollback above them. Reproduced with a real PTY
and a purpose-built VT screen model (`crates/pty/src/screen.rs`).

**The fix** is in `bash/completion.bash`:

- `_mbx_comp_overlay_reserve` makes room *before* anything saves the cursor.
  `\eD` (IND) moves down a row and scrolls at the bottom margin, so N of them
  let the screen absorb the scroll the draw was going to cause; moving back up
  N rows lands on the prompt's row wherever it now is. A `\e7` taken after that
  cannot be invalidated. IND rather than `\n` because IND leaves the column
  alone — `\n` would save the start of the line instead of the user's cursor
  within it, and the dismissing `\e[J` would then erase the prompt text itself.
- `_mbx_comp_overlay_capacity` caps the draw at `LINES - 2`, since reserving
  keeps the save valid but does not stop the reservation scrolling the prompt
  off the top.
- Because the draw can now be smaller than the snapshot,
  `_MBX_COMP_OVERLAY_SHOWN` bounds both cycling and acceptance, so neither can
  reach a row the user never had on screen.

No DSR (`\e[6n`) cursor-row query was needed. The earlier note here that one
was required assumed the cursor row had to be *known*, when it only had to be
made *safe*.

Evidence: `crates/pty/tests/overlay_screen.rs`
(`overlay_near_the_bottom_of_a_short_terminal_leaves_the_prompt_intact`, no
longer `#[ignore]`d and confirmed to fail against the unfixed code) and
`tests/bash/modules.bash` OV-2/OV-3. A `SIGWINCH` while the overlay is visible
was unaffected throughout
(`resize_while_overlay_is_visible_leaves_a_usable_prompt`).

Note when testing this: "is the prompt still visible" does **not** discriminate
the defect. Readline redraws the prompt after a `bind -x` widget returns, so it
comes back either way; the stranded overlay rows are the property that
separates fixed from broken.

`COMP-004` still does not move to `complete`: type-to-filter GUI menus remain
`deferred`, and the overlay has no wide/combining-glyph line-width guard, so a
candidate wider than the terminal can still wrap onto an extra row.

## Goal

Opt-in `MBX_COMP_OVERLAY=1` snapshots ranked completion rows. `\C-x\C-o` toggles a
bounded candidate list below the prompt; `\C-xn`/`\C-xp` move selection while
visible; `\C-x\C-a` accepts; `\C-xj` dismisses (stock `\C-g` `abort` stays).

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx-pty --test completion_harness overlay_lists ranked_accept_works_with_overlay_env -- --nocapture
```
