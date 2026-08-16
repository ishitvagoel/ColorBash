# Multiline, display-width, and resize PTY validation

Date: 2026-08-15. Environment: GNU Bash 5.2.21 on Linux/WSL2 with the genuine
PTY driver in `crates/pty` (tests in `crates/pty/tests/multiline_width.rs`). All
cases run the MBX-enabled interactive shell (`bash/init.bash` with the debug
helper, plain-text mode, Git disabled) unless noted.

This is the `RSH-004` evidence base and the first input for the `PRM-002`
capability/width model.

## Findings

### Two-line prompt renders and stays usable

The MBX prompt (`path` line, then the `> ` input anchor) renders under the real
terminal and accepts commands normally. The full prompt lifecycle (rendering,
typing, execution, next prompt) works in one PTY session.

### Multiline input uses the PS2 continuation prompt

Typing `echo one \` + Enter shows the continuation prompt before the next line;
completing with `two` executes `one two`. The MBX prompt does not interfere with
Readline's multiline parsing or PS2.

### Narrow terminals wrap without breaking input

At a 20-column window the two-line prompt and typed commands wrap; the command
still executes and the next prompt appears. No cursor corruption or dropped
input was observed at these widths.

### Resize mid-line preserves the buffer

Typing a partial line, resizing the window (80x24 to 40x10), and then completing
the line yields the exact expected command output. Readline's SIGWINCH redraw
does not discard in-progress input.

### Wide glyphs in the working directory render correctly

A directory named `测 试 目录` (CJK, with spaces) works end to end: `cd`
succeeds, `pwd` reports the full path, and the prompt shows the compact
`~/测 试 目录` form. Commands still execute. Spaces inside directory names do
not break the pipeline because the prompt path is display data, not executable
text.

### Combining characters do not break input

A directory name using a combining mark (`e\u{301}tude`) remains usable; the
prompt renders and commands execute. This confirms display data with combining
glyphs flows through the sanitizer and PS1 without corrupting Readline.

### Long single-line input wraps without corruption

A ~80-character command at a 30-column window wraps; the shell executes it and
echoes the complete output. Readline owns the redraw; MBX never interferes with
the buffer.

### Narrow wrap with a long typed command (non-DSR, 2026-08-16)

At `cols=20`, a `printf` plus a 26-character `echo` payload longer than the
window width still executes with exact output and a usable next `> ` prompt
(`narrow_wrap_long_command_stays_usable` in `crates/pty/tests/multiline_width.rs`).
No CPR/DSR probe is used.

### Wide-glyph wrap at a narrow window (non-DSR, 2026-08-16)

At `cols=12`, `cd` into a CJK directory whose display width exceeds the window
still leaves the shell usable: a subsequent `printf` executes and the next `> `
prompt appears (`narrow_wrap_wide_glyph_payload_stays_usable` in
`crates/pty/tests/multiline_width.rs`). No CPR/DSR probe is used.

## Limits of this evidence

- **CPR/DSR is forbidden on this harness.** A raw PTY has no cursor-position
  responder; sending `\e[6n` and waiting for CPR would hang. Do not add such
  waits to tests.
- Byte-level cursor-position assertions (exact wrap columns) are not asserted;
  the corpus proves usability and exact command output at known `COLUMNS`, not
  pixel or CPR-reported column math.
- Wide-character column counts are validated through successful round trips and
  the unit display-width model in `crates/cli/src/prompt.rs`, not through CPR.
- Resize is validated mid-line; resize during a fullscreen application or during
  an active foreground job remains release-matrix (`HRD-001`) work.
- Terminal capability negotiation (16/256/true color) is recorded separately in
  `docs/prm-002-color-capability-plan.md`.

## Implications

1. The MBX prompt is compatible with multiline input, wrapping, resize, and
   wide/combining glyphs in the tested window sizes; no Readline augmentation is
   needed for these foundation cases.
2. `PRM-002` has a unit display-width model for path compaction in
   `crates/cli/src/prompt.rs` (`docs/prm-002-width-plan.md`) and non-DSR wrap
   usability probes at known `COLUMNS` (`docs/prm-002-wrap-column-plan.md`);
   exact CPR/DSR column math remains out of scope for a raw PTY harness.
3. Any future ghost-text or popup feature (`G3` onward) must re-use this harness
   for exact-byte and resize evidence; the harness already proves the
   resize-mid-line and narrow-wrap topologies those features depend on.
