# Bash compatibility contract

## Authority

Bash remains the parser, expansion engine, executor, job controller, and source of
truth for exit status. MBX may observe context and change interactive
presentation; it may not reinterpret a command.

## Integration invariants

1. `bash/init.bash` returns immediately when `$-` does not contain `i`.
2. Re-sourcing is idempotent.
3. Existing scalar or array `PROMPT_COMMAND` entries remain in order.
4. The status entering the prompt cycle is captured first and returned after MBX
   callbacks.
5. A pre-existing DEBUG trap is untouched by default; duration timing is opt-in.
6. Helper startup, timeout, malformed response, or exit cannot prevent the next
   prompt from being constructed.
7. MBX does not replace `complete`, `.bash_history`, `cd`, or any Bash builtin.
8. The optional history sidecar records Bash-admitted entries only when
   `MBX_HISTORY=1` and never writes, truncates, or rewrites `.bash_history`.
9. No suggestion or generated text is executed by this foundation.
10. Prompt data from paths, Git, environment, or IPC is treated as untrusted.

## Hook findings

`PROMPT_COMMAND` is the reliable post-command boundary for an interactive prompt.
The first callback can capture `$?`; it must do so before any test, assignment
command, or helper invocation. Returning that same value allows later callbacks
to observe it.

There is no general pre-exec callback. A DEBUG trap fires before simple commands,
functions, substitutions, and prompt callbacks, so it needs a state machine and
still collides with other frameworks. Bash resets or hides DEBUG traps in several
subshell/function contexts, making generic trap composition unsafe. The prototype
therefore chooses no duration rather than overwriting unknown user behavior.

`bind -x` exposes `READLINE_LINE`, `READLINE_POINT`, and related variables to a
shell function. This supports text insertion and an opt-in ghost suffix after the
cursor (ADR 0010). While that suffix is active, Enter is a Readline delete-char +
`accept-line` macro rather than bind -x (M-041). Left / `\C-b`, Home, Up, and
backward-word strip the suffix before their stock motion. Ctrl-X Ctrl-N / Ctrl-P cycle
other prefix matches without accepting them. This is not continuous
highlighting on every keypress.

## Smoke corpus

`tests/bash/corpus.bash` runs in baseline interactive Bash and in an MBX-enabled
interactive Bash. The suite compares semantic markers for:

- variables and quoting;
- `||`, pipelines, loops, functions, and aliases;
- background jobs and `wait`;
- subshells, here-strings, and process substitution;
- arrays and status preservation;
- `set -u` across prompt cycles.

Additional tests confirm that noninteractive sourcing is a no-op, a pre-existing
DEBUG trap is preserved, and a missing helper uses the Bash fallback.

A genuine PTY suite in `crates/pty` covers interactive prompt lifecycle, helper
failure, Ctrl+C, Ctrl+Z, resize, and `stty -g` restoration, plus history
admission characterization, opt-in sidecar recording, and opt-in ghost suffix
insert/accept. Piped interactive
Bash is still not treated as terminal evidence.

Run:

```bash
bash tests/run.bash
bash tests/bash/smoke.bash target/debug/mbx
```

## Required future matrix

The automated local suite is foundation evidence, not the full compatibility
claim. Before MVP release, exercise current stable Bash and supported Bash 5.x on
Linux, WSL, and macOS, with login/nested shells, tmux, SSH, common terminals,
16/256/true color, Unicode-disabled locales, terminal resize, Ctrl+C/Ctrl+Z,
helper crashes, foreground/background jobs, and fullscreen TUIs.

