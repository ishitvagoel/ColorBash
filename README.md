# MBX — Modern Bash Experience

MBX is an experimental interaction layer for Bash. Bash still parses and runs
every command; MBX only enriches the prompt and, when explicitly enabled, records
admitted history in a local sidecar. Editor features remain later phases.

This repository currently implements the foundation prompt slice plus the UI-free
Phase 3A history sidecar:

- a small Bash loader with status-preserving prompt hooks;
- a modular Bash integration with separate protocol, configuration, engine,
  orchestration, fallback, hook, and history-observation boundaries;
- a Rust helper with a thin composition root, narrow injected interfaces, the
  MBX1 prompt protocol, and MBX2 history-record ingestion;
- a compact adaptive prompt with path, Git, failure, duration, SSH, and explicit
  production states;
- an injected Git repository-status provider with capped 50-ms refreshes and a
  bounded one-second warm cache (broader provider families remain deferred);
- bounded automatic coprocess IPC with per-call and process-free Bash fallbacks
  sharing one render deadline;
- an opt-in, local SQLite history sidecar with exclusions, path/count/clear/delete
  controls, and deterministic recent/prefix/cwd search;
- reproducible process, coprocess, and Unix-socket benchmarks;
- focused Rust/Bash module-contract tests, genuine PTY driver tests,
  compatibility smoke tests, and architectural decision records.

The helper bundles SQLite (`rusqlite` with the `bundled` feature) for the history
store. The protocol crate remains dependency-free. History capture stays off
unless `MBX_HISTORY=1`. Ghost suggestions, completion UI, live highlighting, and
enhanced Ctrl+R are not implemented; they wait on remaining `G0` and/or `G3`.

The helper separates CLI parsing, environment capture, application dispatch,
request handling, rendering, providers, history policy/storage, transports, and
telemetry. Its internal `PromptRendering`, `PromptSegmentProvider`,
`RepositoryStatusProvider`, `RequestHandler`, and history ports are the current
extension and test seams. MBX1 keeps prompt flags as a compatible integer on the
wire while Rust uses the typed `PromptFlags` view; Bash computes the matching
flag set once for every rendering path. See
[`docs/architecture.md`](docs/architecture.md) for dependency direction and the
precise limits of the current prompt and history implementations.

## Try the prototype

Requirements: Bash 5.x, Rust 1.85 or newer, and Git for the optional Git segment.

```bash
cargo build --release --workspace
source "$PWD/bash/init.bash"
```

The setup script builds the helper and prints the source command without editing
your shell configuration:

```bash
bash scripts/dev-setup.bash
```

After validation, add this near the end of `.bashrc` using the repository's
absolute path:

```bash
source /absolute/path/to/ColorBash/bash/init.bash
```

## Prototype controls

```bash
MBX_COLOR=never                 # force plain text
MBX_ICONS=never                 # text fallbacks (default auto is also font-safe)
MBX_ICONS=nerd                  # opt in to Nerd Font glyphs
MBX_DISABLE_GIT=1               # omit Git discovery
MBX_DISABLE_RENDERER=1          # use the Bash-only fallback
MBX_IPC_MODE=coprocess          # auto | coprocess | per-call | off
MBX_RENDER_TIMEOUT=0.10         # total native/fallback attempt budget in seconds
MBX_PRODUCTION_CONTEXT=1        # show the prominent production state
MBX_ENABLE_DURATION_TIMING=1    # opt in only when no DEBUG trap is already used
MBX_HISTORY=1                   # opt in to the local history sidecar
MBX_HISTORY_EXCLUDE='git *'     # colon-separated glob exclusions
MBX_LOG=trace                   # helper timing/events; never logs command text
```

`NO_COLOR` and `TERM=dumb` are respected. If the helper is missing or exits, the
shell continues with a Bash-only prompt. No feature executes or rewrites a user
command. History commands (`mbx history path|count|clear|delete` and
`mbx history search recent|prefix|cwd`) also require `MBX_HISTORY=1`. Stored
command text is plaintext local data.

## Verify

```bash
bash tests/run.bash
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-prompt.bash target/release/mbx
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-ipc.bash target/release/mbx
```

The IPC benchmark needs permission to create a local Unix-domain socket. See
[`docs/architecture.md`](docs/architecture.md) for the current recommendation and
the reassessment gate.

## Documentation

Agent work starts with [`AGENTS.md`](AGENTS.md), which requires reading the
cumulative [`MISTAKES.md`](MISTAKES.md) before planning or editing.

- [`docs/roadmap.md`](docs/roadmap.md) — canonical delivery status, gates, and
  next work
- [`docs/solid-hardening-checklist.md`](docs/solid-hardening-checklist.md) —
  completed bounded SOLID findings and validation evidence
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/ux-spec.md`](docs/ux-spec.md)
- [`docs/bash-compatibility.md`](docs/bash-compatibility.md)
- [`docs/protocol.md`](docs/protocol.md)
- [`docs/protocol-mbx2.md`](docs/protocol-mbx2.md)
- [`docs/history-phase3a-contract.md`](docs/history-phase3a-contract.md)
- [`docs/research/bash-readline-investigation.md`](docs/research/bash-readline-investigation.md)
- [`docs/benchmarks/`](docs/benchmarks/)
- [`docs/adr/`](docs/adr/)
- [`CODEX_MODERN_BASH_ARCHITECTURE.md`](CODEX_MODERN_BASH_ARCHITECTURE.md) —
  originating product brief and long-term intent
