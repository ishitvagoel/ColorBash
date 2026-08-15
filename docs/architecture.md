# Foundation architecture

Status: implemented prototype, architecture reassessment required before editor
features.

## Scope

This phase validates the smallest useful vertical slice: interactive Bash calls a
native helper, receives presentation data, and continues to execute commands with
ordinary Bash semantics. It includes a prompt prototype and transport evidence;
it intentionally stops before completion, history, ghost suggestions, or syntax
highlighting.

## System boundary

```text
Readline input
    │
    ▼
Bash parser and execution ───────► programs / jobs / pipelines
    │
    │ PROMPT_COMMAND status + optional DEBUG timing
    ▼
Bash integration ── MBX1 request ──► Rust helper
    │                                  │
    │                                  ├─ sanitize display data
    │                                  ├─ inspect Git (prototype only)
    │                                  └─ render semantic prompt
    ◄──────────── MBX1 response ────────┘
    │
    ▼
PS1 presentation
```

Bash owns parsing, expansion, execution, jobs, traps, exit codes, aliases,
functions, programmable completion, and history. The Rust process cannot execute
the command line. Its current inputs are only prompt context: working directory,
last status, optional duration, and capability flags.

## Repository layout

```text
bash/                 small interactive integration and fallback
crates/protocol/      dependency-free MBX1 message model
crates/cli/           helper, renderer, stdio/socket servers, benchmark client
docs/                 research, UX contract, compatibility contract, ADRs
scripts/              explicit development setup and IPC benchmark
tests/bash/           semantic corpus and integration smoke test
```

## Prompt lifecycle

`init.bash` returns immediately outside interactive Bash. In an interactive shell
it preserves existing `PROMPT_COMMAND` entries by converting them to an array and
placing two MBX callbacks around them:

1. `_mbx_capture_status` runs first, captures `$?`, and returns that same status.
2. Existing prompt callbacks run in their original order.
3. `_mbx_render_prompt` runs last and also returns the captured status.

This prevents MBX from hiding a failure from an existing prompt callback or from
the next interactive expansion. Command-duration timing needs a pre-execution
signal. Bash's `DEBUG` trap is the viable prototype hook, but Bash cannot safely
discover and compose an arbitrary existing DEBUG trap from a sourced file. It is
therefore disabled by default and enabled only with
`MBX_ENABLE_DURATION_TIMING=1`.

## Transport selection

The loader defaults to a Bash `coproc` running `mbx serve --stdio`. The hot path
uses Bash builtins (`printf` and `read`) over retained file descriptors. The write
runs in a short SIGPIPE-isolated Bash subshell so a helper crash cannot terminate
the interactive parent. Every request has a monotonically increasing ID; responses must match the protocol
magic, request ID, and expected type within a 100 ms deadline.

Failure behavior is layered:

1. use the warm coprocess;
2. after a timeout or broken pipe, close it and try one helper invocation;
3. if the binary is unavailable or fails, render the Bash-only fallback.

Measured on the development WSL/Linux environment with the optimized 0.1.0
binary and 1,000 warm ping requests:

| Transport | Mean round trip | Relative note |
| --- | ---: | --- |
| process per call | 1.288 ms | simplest, but startup dominates |
| Bash coprocess | 0.573 ms | includes crash-safe write isolation |
| Unix socket | 0.060 ms | fastest raw IPC; Bash has no native client |

The Unix-socket number measures one persistent Rust client and server. A real
Bash integration would need `socat`, another helper invocation, or a custom
bridge, erasing its complexity advantage. ADR 0004 therefore selects a coprocess
for the MVP and keeps a secured Unix socket as a daemon option only if later
cross-session workloads justify it.

## Prompt renderer

The renderer uses semantic theme roles internally and produces Bash PS1-safe
escape sequences. Its hierarchy is:

1. explicit production state, otherwise SSH state;
2. compact working path;
3. Git branch and staged/modified/untracked counts;
4. non-zero exit status;
5. duration when at least two seconds.

Repository-controlled branch/path/host text is bounded and strips control
characters, `$`, backticks, and backslashes before entering PS1. Git runs with
color disabled and `GIT_OPTIONAL_LOCKS=0`; output is size-bounded. No repository
file is sourced or executed.

The current Git lookup is synchronous and deliberately visible as prototype
debt. It is acceptable for architecture validation, not for per-keystroke work.
A warm-process TTL cache and asynchronous refresh must precede richer Git status.

## Compatibility and degradation

- Noninteractive shells are untouched.
- Existing `PROMPT_COMMAND` entries remain ordered.
- Existing DEBUG traps remain untouched by default.
- `NO_COLOR`, `TERM=dumb`, plain text, and no-Nerd-Font modes are supported.
- Coprocess failure falls back without disabling the shell.
- The helper has no third-party runtime or crate dependencies.
- No MBX path evaluates user/repository text as Bash.

## Reassessment gate

The foundation supports continuing, with conditions. Before autocomplete or live
highlighting:

1. run PTY tests under tmux, SSH, WSL, Linux, and macOS Bash variants;
2. add a warm Git cache and measure real prompt render p50/p95/p99;
3. test prompt callback ordering against popular prompt/preexec frameworks;
4. prototype one non-destructive `bind -x` editing action that only inserts text;
5. prototype reuse of one standard completion function and document quoting,
   `compopt`, and dynamic-completion failures;
6. decide whether duration timing remains opt-in or integrates with an existing
   preexec framework through an explicit adapter.

The next coherent slice should be the history sidecar, because it can be built and
tested without taking ownership of Readline redraw. Syntax highlighting and an
interactive completion popup remain blocked on the editor integration experiment.
