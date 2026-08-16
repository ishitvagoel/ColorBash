# Foundation architecture

Status: implemented prototype with the foundation prompt slice plus a UI-free,
opt-in history sidecar. Editor features still require the reassessment work
described below.

## Scope

The implemented prototype is the hybrid Bash/Rust prompt path plus the Phase 3A
history sidecar. Interactive Bash calls a native helper, receives presentation
data, and continues to execute commands with ordinary Bash semantics. When
explicitly enabled, Bash also observes admitted history and enqueues records
without modifying `.bash_history`. Ghost suggestions, completion UI, live
highlighting, and enhanced Ctrl+R remain gated.

## System boundary

```text
Readline input
    │
    ▼
Bash parser and execution ───────► programs / jobs / pipelines
    │
    │ PROMPT_COMMAND status + optional DEBUG timing
    ▼
Bash integration ── MBX1 PROMPT ──► Rust helper
    │                                  │
    │                                  ├─ validate and dispatch request
    │                                  ├─ inspect Git through bounded/cache adapter
    │                                  ├─ assemble semantic prompt segments
    │                                  └─ sanitize and render presentation
    ◄──────────── MBX1 response ────────┘
    │
    │ optional MBX2 RECORD (MBX_HISTORY=1)
    └──────────────────────────────► helper writer queue ──► local SQLite
    │
    ▼
PS1 presentation
```

Bash owns parsing, expansion, execution, jobs, traps, exit codes, aliases,
functions, programmable completion, and history. The Rust process cannot execute
the command line. Prompt rendering still takes only prompt context: working
directory, last status, optional duration, and capability flags. History capture
is a separate opt-in path that treats command text as inert data.

## Repository and module boundaries

```text
bash/
  init.bash          composition root and module loading
  protocol.bash      MBX1/MBX2 constants, field codec, response validation
  config.bash        environment-to-flag and transport-selection policy
  engine.bash        coprocess lifecycle and native transport adapters
  prompt.bash        fallback orchestration; the only prompt-path PS1 writer
  fallback.bash      Bash-only prompt renderer
  hooks.bash         PROMPT_COMMAND and optional DEBUG integration
  history.bash       opt-in admitted-entry observation and MBX2 RECORD send

crates/protocol/     dependency-free MBX1 wire model and PromptFlags value type
crates/cli/src/
  main.rs            process exit-code adapter
  lib.rs             Rust composition root
  cli.rs             side-effect-free argument parsing
  environment.rs     one-time process environment capture
  app.rs             top-level command/use-case dispatch
  service.rs         transport-independent MBX1 request handling
  history.rs         history ports, entry type, and drop-rule validation
  history_service.rs MBX2 RECORD/PING handling behind HistoryHandler
  policy.rs          opt-in and exclusion policy from the environment
  prompt.rs          prompt policy, segments, theme, and sanitization
  provider.rs        repository-status port and current Git adapter
  storage.rs         SQLite schema, WAL writer queue, queries, and controls
  transport.rs       stdio/Unix-socket server and client adapters
  telemetry.rs       opt-in trace output

crates/pty/          genuine POSIX PTY test driver; not a product path
docs/                research, UX/compatibility contracts, and ADRs
scripts/             explicit development setup and prompt/IPC/history benchmarks
tests/bash/           module contracts, semantic corpus, and integration smoke
```

The source-file split is a boundary, not merely an organizational convention.
The Rust binary delegates immediately to the library composition root. The Bash
loader only resolves paths, sources modules in dependency order, selects the
binary, starts the engine, and installs prompt and optional history hooks.

## Dependency direction

The stable protocol types sit at the shared boundary. Within the helper,
orchestration and I/O depend on narrow application ports; concrete adapters are
selected only at the composition boundary.

```text
main.rs ──► lib.rs composition root
               │
               ├─► CLI/environment adapters
               ├─► PromptRenderer ──► PromptSegmentProvider
               │          │
               │          └─► RepositoryStatusProvider ◄── Git adapter
               │
               └─► app ──► ProtocolService ──► PromptRendering
                         │
                         ├─► transport ─────► RequestHandler
                         └─► HistoryService ──► HistoryRecorder
                                           └──► HistoryPolicy
               History CLI ──► HistorySearch / HistoryControl

crates/cli ──► crates/protocol
crates/cli ──► rusqlite (bundled; history store only)
```

The important production dependency direction is toward abstractions:

- transport entry points accept a `RequestHandler`; they do not require a
  `ProtocolService` concrete type.
- `ProtocolService` accepts `PromptRendering` and does not construct a renderer.
- `ProtocolService` maps the MBX1 `PromptRequest` DTO into the
  transport-independent `PromptContext` before calling the renderer.
- the repository segment accepts `RepositoryStatusProvider` and does not spawn
  Git itself.
- `lib.rs` constructs `GitRepositoryStatusProvider` and injects it into the
  standard renderer.
- history CLI and MBX2 handling depend on `HistoryPolicy`, `HistoryRecorder`,
  `HistorySearch`, and `HistoryControl`; `app.rs` opens the store only after
  `MBX_HISTORY=1` is established.
- `crates/protocol` has no dependency on the CLI crate or its adapters.

`app.rs` remains an outer application shell: it owns stdout flushing, server
selection, socket-client commands, history-control dispatch, and benchmark
dispatch. Prompt policy and request handling remain usable without those
process-level effects.

## SOLID application and extension seams

The refactor applies SOLID as design guidance rather than treating each file as
an independent subsystem:

- **Single responsibility:** argument parsing, environment capture, request
  dispatch, rendering, repository inspection, history policy/storage, transport,
  and telemetry have separate change boundaries. Bash likewise separates
  protocol, policy, lifecycle, orchestration, rendering, hooks, and history
  observation.
- **Open/closed:** a prompt capability can implement `PromptSegmentProvider` and
  be added to the renderer's ordered provider list. A different repository
  implementation can implement `RepositoryStatusProvider` without changing the
  repository segment.
- **Liskov substitution:** direct request-handler, renderer, repository, and
  history substitutes exercise success and failure contracts. Provider
  absence/failure omits only its segment, disabling Git never calls the
  provider, and a disabled history policy never opens the store.
- **Interface segregation:** `PromptRendering`, `PromptSegmentProvider`,
  `RepositoryStatusProvider`, `RequestHandler`, and the history ports each
  expose the operations needed by their consumer.
- **Dependency inversion:** services and adapters receive those ports; the
  composition root is responsible for choosing concrete implementations.

The main extension and test seams are:

| Concern | Port or seam | Current implementation/test substitute |
| --- | --- | --- |
| request handling | `RequestHandler` | `ProtocolService`; direct transport stubs prove envelope ownership and bounds |
| prompt rendering | `PromptRendering` | `PromptRenderer`; service tests inject a stub renderer |
| prompt composition | `PromptSegmentProvider` list | ordered built-in segments or injected providers |
| repository state | `RepositoryStatusProvider` | Git adapter or an in-memory static provider |
| history policy | `HistoryPolicy` | `EnvironmentHistoryPolicy` or allow/deny substitutes |
| history record | `HistoryRecorder` | `QueuedHistoryStore` or recording substitutes |
| history search | `HistorySearch` | SQLite reader or in-memory substitutes |
| history controls | `HistoryControl` | path/count/clear/delete on the same store |
| MBX2 handling | `HistoryHandler` | `HistoryService`; transport injects it only when history is enabled |
| stream exchange | generic `BufRead`/`Write` in `ClientSession` | Unix streams or in-memory cursors |
| CLI defaults | injected lazy defaults resolver | captured environment or explicit test values |
| Bash adapters | arguments plus `REPLY` result | real engine/fallback or focused function tests |

These are internal seams, not a public extension API or general plugin system.
They are deliberately narrow interfaces for the implemented prompt and history
slices.

The bounded post-refactor SOLID audit is implemented and recorded in
`docs/solid-hardening-checklist.md`: transport owns correlation/framing,
`RequestHandler` returns content only, Git acquisition is capped and deadline-
controlled beneath the pure parser, and all prompt adapters share one explicit
context/flag/safety/liveness contract. Semantic prompt composition versus trusted
PS1 encoding remains `PRM-009` discovery until capability/width work or a second
renderer proves that change axis; a speculative extra abstraction is not required
for the current slice.

## PromptFlags boundary

MBX1 keeps the wire field as an additive `u32` for compatibility. The protocol
crate now exposes `PromptFlags`, a typed view with named queries and mutations.
It preserves unknown bits when flags are decoded and modified, allowing older
peers to forward newer capabilities without understanding them.

Rust CLI parsing, environment capture, and rendering use this abstraction instead
of scattering raw bit operations. Bash mirrors the same named bit constants
because it must construct the wire value without a Rust runtime. `config.bash`
computes the flags once per prompt cycle. The raw value is passed to the coprocess
and fallback; per-call mode passes the same value through `mbx prompt --flags
<u32>`. Later named CLI switches modify known bits without discarding unknown
additive bits.

## Bash prompt lifecycle

`init.bash` returns immediately outside interactive Bash. In an interactive shell
it sources the modules in their dependency order and preserves existing
`PROMPT_COMMAND` entries by converting them to an array and placing two MBX
callbacks around them:

1. `_mbx_capture_status` runs first, captures `$?`, optionally records the last
   admitted history entry, and returns that same status.
2. Existing prompt callbacks run in their original order.
3. `_mbx_render_prompt` runs last and also returns the captured status.

`prompt.bash` creates one explicit context (`status`, `duration`, `cwd`, and
flags), chooses an adapter, and commits the selected result to `PS1`. The native
coprocess adapter, per-call adapter, and Bash fallback return their candidate via
`REPLY`; none writes `PS1` directly. This makes fallback order testable without
installing interactive hooks. History observation never writes `PS1`.

Command-duration timing needs a pre-execution signal. Bash's `DEBUG` trap is the
viable prototype hook, but Bash cannot safely discover and compose an arbitrary
existing DEBUG trap from a sourced file. It is therefore disabled by default and
enabled only with `MBX_ENABLE_DURATION_TIMING=1`.

## Transport selection

The loader defaults to a Bash `coproc` running `mbx serve --stdio`. The hot path
uses Bash builtins (`printf` and `read`) over retained file descriptors. The write
runs in a short SIGPIPE-isolated Bash subshell so a helper crash cannot terminate
the interactive parent. Each request has a monotonically increasing ID, and the
response decoder checks the protocol magic, correlation ID, field count, and
expected response type.

One `MBX_RENDER_TIMEOUT` deadline, defaulting to 0.10 seconds, covers outbound
encoding, coprocess exchange/decoding/cleanup, per-call fallback, and final
selection. `MBX_IPC_TIMEOUT` can impose a smaller cap on the coprocess exchange
but cannot grant a second budget. Outbound framing preflights the encoded size;
response acquisition detects raw NUL and reads bounded chunks into at most the
65,536-byte payload plus the CRLF allowance before decoding. Percent decoding and
field splitting cooperatively check the same deadline.

Failure behavior is layered:

1. use the warm coprocess;
2. after a timed-out read, broken pipe, invalid response, or dead child, close it
   and try one helper invocation only with the remaining budget;
3. if the binary is unavailable, invalid, or out of time, render the process-free
   Bash fallback.

Timed-out children receive `TERM`/`KILL`, their descriptors close immediately,
and reaping is deferred until a later prompt can prove `wait` will not block.
Deadline enforcement is cooperative between bounded Bash builtins, so an
in-progress 4-KiB read or native pattern operation can overshoot by a small amount
without becoming unbounded.

Measured on the development WSL/Linux environment with the optimized 0.1.0
binary and 1,000 warm ping requests:

| Transport | Mean round trip | Relative note |
| --- | ---: | --- |
| process per call | 1.068 ms | simplest, but startup dominates |
| Bash coprocess | 0.500 ms | includes crash-safe write isolation |
| Unix socket | 0.048 ms | fastest raw IPC; Bash has no native client |

The Unix-socket number measures one persistent Rust client and server. Full
output and a warm-Git prompt percentile workload are in
`docs/benchmarks/2026-08-15-solid-hardening.md`. A real Bash integration would
need `socat`, another helper invocation, or a custom bridge, erasing its
complexity advantage. ADR 0004 therefore selects a coprocess
for the MVP and keeps a secured Unix socket as a daemon option only if later
cross-session workloads justify it. The experimental socket server still handles
clients sequentially, and an abrupt process signal can leave its `0600` socket
path for the operator to verify and remove; robust daemon lifecycle handling is
not part of this refactor.

## Prompt renderer and current provider

The renderer asks ordered `PromptSegmentProvider` implementations for semantic
segments, sanitizes all returned text centrally, then applies the theme. Its
default hierarchy is:

1. explicit production state, otherwise SSH state;
2. compact working path;
3. Git branch and staged/modified/untracked counts;
4. non-zero exit status;
5. duration when at least two seconds.

Repository-controlled branch/path/host text is bounded to 256 characters and
strips control characters, `$`, backticks, and backslashes before entering PS1.
The current `RepositoryStatusProvider` is the bounded prompt slice of ADR 0007.
At composition time it resolves Git only from executable files in absolute
`PATH` entries and stores the absolute program path; empty/relative entries and a
bare-command fallback are rejected. A fixed worktree preflight and fixed `git
status` command share a maximum 50-ms budget. Color, filesystem monitoring,
terminal prompting, and optional locks are disabled. Stdout is acquired through a
1-MiB-plus-one capped reader, and parser output remains typed and centrally
sanitized.

A 128-entry, one-second TTL cache stores the complete provider outcome (`Some`,
`None`, or typed `Err`) and supports explicit invalidation. Prompt degradation
omits only the Git segment, while trace output exposes only the typed failure kind
and never command text. The controlled warm-Git workload measured p50/p95/p99 of
718/974/1,383 us.

On the normal timeout path, the process runner attempts and tests direct-child
kill/reap; failures become typed cleanup errors. It does not claim portable
process-tree termination: an unexpected descendant holding inherited stdout can
outlive the call, but the detached capped reader cannot delay prompt return.
Absolute `PATH` entries are trusted configuration, kernel-level
`spawn`/`kill`/`wait` stalls are not independently cancellable, and a rare fatal
preflight failure is treated as absence because stderr is deliberately not
acquired. Generic detection/completion/diagnostic providers remain deferred until
a concrete consumer establishes their contracts.

## Opt-in history sidecar

The Phase 3A sidecar is implemented and stays off unless `MBX_HISTORY=1`. Bash's
admitted history list is the capture authority: `history.bash` reads `history 1`
at the prompt boundary after command completion (skipping the first prompt),
drops empty or excluded entries, and sends an MBX2 `RECORD` over the existing
coprocess with its own bounded deadline. The diagnostic `history_number` is the
`history 1` list number, not `HISTCMD`. Failure, queue saturation, or a missing
helper drops enhancement data only and must not block prompt construction.

The helper opens `$XDG_DATA_HOME/mbx/history.sqlite3` (falling back to
`$HOME/.local/share/mbx/history.sqlite3`) with directory mode `0700` and file
mode `0600`. A bounded in-process queue acknowledges enqueue; a per-session
writer commits schema v2 in WAL mode (forward-only migration from v1; see ADR
0008), idle-flushes partial batches when the queue is empty while keeping
`WRITER_BATCH_SIZE=32` for busy ingest, applies retention after full batches
and shutdown, and treats `(session_id, event_sequence)` as the idempotency key.
`ACK` means the record was accepted by the queue, not that SQLite has committed.
Search is a direct CLI operation (`mbx history search recent|prefix|cwd`), not
an MBX2 query.
`path`, `count`, `clear`, and `delete` are the privacy controls. Command text
never enters traces.

This slice does not enable history-driven UI. Invariance and admission-parity
PTY evidence is in `crates/pty/tests/history_invariance.rs`. 100k query p95 and
hostile inertness evidence is in `docs/benchmarks/2026-08-16-history-queries.md`
and `crates/cli/src/corpus.rs`. Prompt-boundary write-ack PTY and release
percentile evidence is in `crates/pty/tests/history_write_ack.rs` and
`docs/benchmarks/2026-08-16-history-write-ack.md` (correctness recorded;
percentile budget still open on development WSL). WAL crash/corrupt recovery
and WAL/SHM `0600` never-more-permissive evidence are in
`crates/cli/src/storage.rs` (`docs/history-g2-wal-crash-plan.md`,
`docs/history-g2-permission-plan.md`). Many-match prefix covering-index evidence
is in `crates/cli/src/storage.rs` (Q-A–Q-C) and
`docs/benchmarks/2026-08-16-history-prefix.md`. Writer idle-flush for live
`count`/`search` evidence is in `crates/cli/src/storage.rs` (V-1–V-2) and
`crates/pty/tests/history_invariance.rs` (V-3). `G2` still requires
prompt-boundary write-ack budget pass and foreign-user open.

## Compatibility and degradation

- Noninteractive shells are untouched.
- Existing `PROMPT_COMMAND` entries remain ordered.
- Existing DEBUG traps remain untouched by default.
- `NO_COLOR`, `TERM=dumb`, plain text, and no-Nerd-Font modes are supported.
- Coprocess failure falls back without disabling the shell.
- Native and fallback rendering replace the complete C0/DEL range and Bash
  expansion characters using a shared hostile-state corpus.
- The protocol crate has no third-party dependencies. The helper bundles
  `rusqlite` for the history store (ADR 0005 section 6a) and otherwise uses the
  standard library.
- No MBX path evaluates user or repository text as Bash.
- The history sidecar never writes, truncates, or rewrites `.bash_history`.

## Reassessment gate

The foundation supports continuing, with conditions. Before autocomplete or live
highlighting:

1. run PTY tests under tmux, SSH, WSL, Linux, and macOS Bash variants;
2. extend the controlled warm-Git measurement into representative dirty/large,
   cold-refresh, fallback, PTY, and platform p50/p95/p99 workloads;
3. test prompt callback ordering against popular prompt/preexec frameworks;
4. prototype one non-destructive `bind -x` editing action that only inserts text;
5. prototype reuse of one standard completion function and document quoting,
   `compopt`, and dynamic-completion failures;
6. decide whether duration timing remains opt-in or integrates with an existing
   preexec framework through an explicit adapter.

The roadmap, not this architecture description, selects the next slice. A genuine
PTY driver now covers foundation prompt lifecycle, helper failure, Ctrl+C,
Ctrl+Z, resize, `stty -g` restoration, multiline continuation, narrow wrap,
resize-mid-line, and wide/combining glyph round trips (`docs/research/
multiline-width-pty.md`), and the Bash history admission corpus
(`docs/research/bash-history-admission.md`). The opt-in history sidecar is
implemented; `G2` budgets and invariance evidence remain. Provider expansion,
highlighting, and completion remain gated. Editor-facing work still requires
`G3`/`G4`.
