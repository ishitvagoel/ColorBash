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

## Limits of this evidence

- Byte-level cursor-position assertions (exact wrap columns) are not yet
  asserted; the corpus proves usability and exact command output, not pixel or
  column math.
- Wide-character column counts (East Asian width) are validated only through
  successful round trips, not through a display-width model. A true width model
  is still `PRM-002` design work.
- Resize is validated mid-line; resize during a fullscreen application or during
  an active foreground job remains release-matrix (`HRD-001`) work.
- Terminal capability negotiation (16/256/true color) is out of scope here.

## Implications

1. The MBX prompt is compatible with multiline input, wrapping, resize, and
   wide/combining glyphs in the tested window sizes; no Readline augmentation is
   needed for these foundation cases.
2. `PRM-002` now has a unit display-width model for path compaction in
   `crates/cli/src/prompt.rs` (`docs/prm-002-width-plan.md`): East Asian wide
   glyphs and combining marks count as display columns, the two-line anchor is
   unchanged, and wrap math still needs PTY column probes rather than byte
   counts.
3. Any future ghost-text or popup feature (`G3` onward) must re-use this harness
   for exact-byte and resize evidence; the harness already proves the
   resize-mid-line and narrow-wrap topologies those features depend on.
