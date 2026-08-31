# ADR 0015: Preview-row syntax highlighting (supersedes in-buffer markers)

Status: Accepted (2026-08-31)

## Context

ADR 0013 painted styled text into `READLINE_LINE` using Readline
non-printing markers (`\001`/`\002`). That is their documented `PS1`
behavior. It is not their behavior inside the edit buffer: Readline
caret-renders the markers as `^A`/`^[`/`^B` (`M-064`). Live color has
therefore never displayed correctly; `color=0` on the interactive refresh
was a deliberate safe state after `M-062`.

Readline exposes no zero-width marker for the edit buffer. Taking
redisplay ownership (Strategy B) is still unjustified (ADR 0003). The
completion overlay already proved that rows reserved *below* the prompt
with IND (`\eD`) survive Readline's post-widget redraw (`M-065`).

Cursor units also disagree today: Bash `READLINE_POINT` and `${#plain}`
are character offsets; `crates/cli/src/highlight.rs` mapped them as UTF-8
byte offsets. That bug is masked by `color=0`.

## Decision

1. **`READLINE_LINE` stays permanently plain.** Wrapped insert, delete, and
   motion update the ordinary edit buffer. Executed bytes are the buffer;
   the Enter restore macro, `_MBX_HIGHLIGHT_ACTIVE` styled-buffer flag, and
   styled-point mapping are removed.
2. **Paint the helper's styled copy on one reserved row below the prompt**,
   using the M-065 reservation (IND, then DECSC, draw, DECRC). Cap the
   painted row at `COLUMNS-1` display columns (SGR skipped; non-ASCII
   counted as two columns). Helper failure or a missing tty leaves the
   plain buffer unpainted.
3. **Color is `_mbx_highlight_color_flag`**, passed as `HIGHLIGHT.color` /
   `--color` (M-062). Paint goes to `/dev/tty`; `bind -x` widgets often have
   stdout as a pipe, so `_mbx_color_capable`'s `-t 1` check would keep live
   color off. Honor `TERM`/`NO_COLOR`/`MBX_COLOR`, then `-t 1 || -t 0 ||
   -w /dev/tty`. Markers in helper output are stripped before the tty write;
   SGR remains. Unexpected C0 after marker-strip is refused — except ESC,
   which SGR requires. Do not skip C0 with an octal glob range that includes
   ESC (`$'\030'-$'\037'` is 24–31; ESC is octal 033).
4. **Point units are Unicode scalar counts** on the HIGHLIGHT/CLI `point`
   field, matching Bash `${#var}` / `READLINE_POINT` in a UTF-8 locale.
   The helper converts to byte offsets internally. The interactive preview
   ignores the returned styled point; the cursor stays on the plain line.
5. **Mutual exclusion with the completion overlay.** While
   `_MBX_COMP_OVERLAY_VISIBLE=1`, highlight does not paint (the overlay
   owns the rows below the prompt). Dismissing the overlay allows the next
   highlight refresh to paint.
6. Ghost+highlight remain mutually exclusive (ADR 0013 decision 5).

ADR 0013's install rules (opt-in, tty, occupied-binding skip, no Tab
rebind) and ADR 0014's coprocess HIGHLIGHT/STYLED frames are unchanged.
Only the rendering site moves.

## Alternatives

- Descope live color, keep `mbx highlight` CLI only: rejected; the preview
  row is evidenced by M-065 and does not take redisplay ownership.
- Custom editor / after-every-key paint hook: still unjustified (ADR 0003).
- Keep in-buffer markers and hope for a Readline patch: no portable
  technique exists; PTY evidence forbids it.

## Consequences

`HLT-002` can leave `blocked` only until PTY evidence shows the preview
row, an intact prompt, exact plain bytes on Enter, and helper-failure
degradation. That evidence landed 2026-08-31
(`highlight_preview_row_paints_sgr_below_an_intact_prompt`). `HLT-003` p99
stays deferred. `COMP-004` overlay width guard is a sibling of the same
below-prompt paint budget.

## Validation

- Module: refresh leaves `READLINE_LINE` equal to the plain bytes; a
  nonzero helper still does not mutate the line.
- PTY + `Screen`: after typing with `MBX_HIGHLIGHT=1` and a color-capable
  tty, the modelled screen contains an SGR-styled copy of the typed text
  on a row below the prompt; the prompt line is intact; Enter executes
  the exact plain bytes.
- `cargo test -p mbx highlight::` for character-offset point mapping.
