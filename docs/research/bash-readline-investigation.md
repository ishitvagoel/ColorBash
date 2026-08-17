# Bash lifecycle, Readline, and rendering investigation

Date: 2026-08-15. Local validation environment: Bash 5.2.21 on Linux/WSL.

## Bash lifecycle

### After execution

`PROMPT_COMMAND` runs before Bash displays each primary prompt. Bash accepts a
scalar command string or an array whose entries run in order. The status of the
foreground command is available to the first entry as `$?`; even a preliminary
`[[ ... ]]` would destroy it. A capture function can return the captured status so
the next callback sees the original value.

`PS1` is expanded after `PROMPT_COMMAND`. Color control sequences must be enclosed
in `\[` and `\]` or Readline miscalculates cursor position. Because Bash may perform
parameter and command substitution on PS1 (`promptvars`), untrusted data cannot be
placed there verbatim.

### Before execution

Bash has no dedicated pre-exec hook. The DEBUG trap fires before simple commands,
functions, arithmetic/substitution contexts, and prompt callbacks. A usable
pre-exec adapter needs an “at prompt / in prompt cycle” state machine. It must also
account for `set -T`/`functrace`, where DEBUG is inherited into functions.

Generic DEBUG trap composition is unsafe. `trap -p DEBUG` observed from command
substitution, an ordinary called function, or some sourced contexts may see reset
trap state rather than the caller's trap. Evaluating another framework's trap text
would introduce quoting and semantic risks. MBX therefore leaves timing disabled
unless the user explicitly opts in and confirms no DEBUG trap is in use.

`PS0` expands after a complete command is read and before it executes, but command
substitution in PS0 runs in a subshell and cannot update parent-shell timing state.
Other parameter-expansion assignment tricks produce visible prompt text. It is not
a clean general replacement.

## Readline constraints

`bind -x` can attach a key sequence to a shell function. During that function,
Bash exposes:

- `READLINE_LINE`: current buffer, writable;
- `READLINE_POINT`: cursor byte index, writable;
- `READLINE_MARK`: selection mark where supported;
- `READLINE_ARGUMENT`, `READLINE_KEYSEQ`: contextual key data.

This is suitable for explicit actions such as accepting a suggestion or inserting
a selected command. When the function returns, Readline redraws the line. It does
not provide a supported “decorate these token ranges” API, and Bash does not call a
shell hook after every ordinary editing key. Rebinding every printable key would
conflict with user keymaps, macros, vi/emacs modes, bracketed paste, and terminal
escape sequences.

Readline already owns wrapping, multiline cursor movement, redisplay, terminal
resize, and bracketed paste. `PS2` can style continuation prompts, but a custom
multiline gutter or continuous highlighting requires deeper editor integration.

Safe initial keys must be configurable and installed only after inspecting current
bindings. The future implementation should expose commands first and let users
opt into defaults where collisions exist.

## Programmable completion reuse

Inside a completion function Bash supplies `COMP_LINE`, `COMP_POINT`,
`COMP_WORDS`, `COMP_CWORD`, `COMP_TYPE`, and `COMP_KEY`; the function populates
`COMPREPLY` and may call `compopt`. `complete -p NAME` can identify an existing
specification, but reuse is not merely calling a function:

- `-F`, `-C`, `-W`, `-A`, glob, and directory actions behave differently;
- completion functions expect exact dynamic variables and may inspect shell state;
- `compopt` changes quoting, spacing, filenames, and fallback behavior;
- command substitutions can be expensive or execute external tools;
- aliases, `sudo`-style wrappers, redirections, `--`, and incomplete quoting alter
  tokenization;
- `COMPREPLY` is strings, so descriptions and kinds require a side channel/provider;
- inserting a normalized candidate must preserve Bash's quoting decisions.

A future adapter should first wrap one known `_completion_loader`/`-F` path in an
isolated experiment, snapshot the dynamic variables, and compare inserted text
with stock Bash. It should fall through to the original completion on any unknown
specification rather than degrading it.

## Rendering and Unicode

Readline must remain the cursor/redisplay authority in the augmentation strategy.
ANSI data from Git branches, filenames, providers, or error messages must be
removed before rendering. Visible-width calculations eventually need grapheme and
terminal-width handling rather than byte or Unicode-scalar counts; emoji variation
selectors, combining marks, East Asian width, and terminal disagreement make this
nontrivial.

The prompt prototype avoids width-dependent right prompts and popups. It uses
font-safe text by default and bounds long path data. A completion popup or ghost
text must handle `SIGWINCH`, wrapped lines, multiline buffers, and redisplay after
Ctrl+C before it can be considered safe.

## Experimental conclusion

Use Readline augmentation for the MVP, a Bash coprocess for the warm helper, and a
strict fallback path. Validate history and explicit `bind -x` insertions next.
Defer continuous highlighting and popup ownership until a focused advanced-editor
prototype proves terminal restoration and stock completion parity.

### Insert-time redraw evidence (2026-08-16)

`bind -x` insertions in `bash/editor.bash` redraw through Readline without
rebinding printable keys. PTY cases E-1–E-4, M-1–M-4, and B-1–B-4 in
`crates/pty/tests/editor_bind_x.rs` show mid-line, quoted, and `PS2` continuation
buffers preserve exact bytes and usable prompts after insert, cancel, resize, and
job control. This satisfies the **insert-time** redraw question for explicit
actions only. Continuous syntax decoration, ghost text, and popup menus still
have no supported after-every-key hook; they remain deferred per ADR 0003 until a
separate strategy is evidenced (`docs/edt-001-exact-bytes-plan.md` B-5). Opt-in
inline ghost (ADR 0010) keeps the suggestion in `READLINE_LINE` after
`READLINE_POINT` instead of painting after redisplay. Enter is a Readline-only
kill-line + accept-line macro while that suffix is active (M-041).

