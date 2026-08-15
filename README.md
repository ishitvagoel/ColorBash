# MBX — Modern Bash Experience

MBX is an experimental interaction layer for Bash. Bash still parses and runs
every command; MBX only enriches the prompt and, in later phases, the editing
experience.

This repository currently implements the architecture-discovery slice from the
project brief:

- a small Bash loader with status-preserving prompt hooks;
- a dependency-free Rust helper and versioned protocol;
- a compact adaptive prompt with path, Git, failure, duration, SSH, and explicit
  production states;
- automatic coprocess IPC with per-process and Bash-only fallbacks;
- reproducible process, coprocess, and Unix-socket benchmarks;
- Bash compatibility smoke tests and architectural decision records.

Completion, history, autosuggestions, and live highlighting are deliberately not
implemented yet. The foundation must be validated before those features begin.

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
MBX_PRODUCTION_CONTEXT=1        # show the prominent production state
MBX_ENABLE_DURATION_TIMING=1    # opt in only when no DEBUG trap is already used
MBX_LOG=trace                   # helper timing/events; never logs command text
```

`NO_COLOR` and `TERM=dumb` are respected. If the helper is missing or exits, the
shell continues with a Bash-only prompt. No feature executes or rewrites a user
command.

## Verify

```bash
bash tests/run.bash
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-ipc.bash target/release/mbx
```

The benchmark needs permission to create a local Unix-domain socket. See
[`docs/architecture.md`](docs/architecture.md) for the current recommendation and
the reassessment gate.

## Documentation

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/ux-spec.md`](docs/ux-spec.md)
- [`docs/bash-compatibility.md`](docs/bash-compatibility.md)
- [`docs/protocol.md`](docs/protocol.md)
- [`docs/research/bash-readline-investigation.md`](docs/research/bash-readline-investigation.md)
- [`docs/adr/`](docs/adr/)
