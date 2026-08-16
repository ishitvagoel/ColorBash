# MBX — Modern Bash Experience

MBX is an experimental interaction layer for Bash. Bash still parses and runs
every command; MBX only enriches the prompt and, when explicitly enabled, records
admitted history in a local sidecar. Suggestions never execute automatically.

## What you can try now

These slices have working code you can exercise in an interactive shell:

| Feature | How it is enabled | Notes |
| --- | --- | --- |
| Adaptive prompt | `source bash/init.bash` | Path, Git, failure, SSH, production |
| Command duration | `MBX_ENABLE_DURATION_TIMING=1` | Opt-in; prompt shows elapsed time at ≥ 2 s |
| History sidecar | `MBX_HISTORY=1` | Local SQLite; never rewrites `.bash_history` |
| Insert token (`bind -x`) | Default chord `Ctrl-X Ctrl-Y` | Inserts text; does not run it |
| Stock Tab completion | Always | File/`-F` insertion stays Bash; no popup yet |
| Wrapped `-F` metadata | `_mbx_comp_wrap_existing_f NAME` | Additive kinds/scores; Tab bytes unchanged |
| Ranked-accept chord | Default `Ctrl-X Ctrl-A` after wrapped Tab | Replaces current word with ranked candidate; Tab stays stock |
| Git completion kinds | Wrap `git` or `mbx_comp_git` fixture | Additive `ref`/`flag`/`file`; no Git subprocess |
| Fuzzy history search | `MBX_HISTORY=1` then `mbx history search fuzzy TEXT` | Ranks a bounded recent pool |
| Failed history search | `MBX_HISTORY=1` then `mbx history search failed` | Rows with nonzero exit status |

## What remains

These MVP features are **not** implemented for interactive use:

| Feature | Why it is waiting |
| --- | --- |
| Ghost suggestions | No after-every-key Readline decoration hook |
| Completion popup | Overlay unproven; ranked-accept chord exists |
| Syntax highlighting | Same continuous-decoration leftover |
| Enhanced Ctrl+R | Same leftover; explicit search UI not built |
| Repository-context history | `HIST-010` |
| macOS PTY matrix | `HRD-001` needs a macOS host |

Canonical status lives in [`docs/roadmap.md`](docs/roadmap.md). `G0`, `G2`,
`G3`, and `G4` are complete. Continuous decoration stays unproven (ADR 0003).

The helper bundles SQLite (`rusqlite` with the `bundled` feature) for the history
store. The protocol crate remains dependency-free. History capture stays off
unless `MBX_HISTORY=1`.

The helper separates CLI parsing, environment capture, application dispatch,
request handling, rendering, providers, history policy/storage, transports, and
telemetry. See [`docs/architecture.md`](docs/architecture.md) for dependency
direction and the precise limits of the current prompt, history, editor, and
completion implementations.

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

Use a real terminal (not a pipe). A piped Bash process is not PTY evidence.

## Manual tests

Run each scenario after the `source` line above. None of these execute a
suggestion on your behalf.

### 1. Prompt

```bash
cd ~
false
```

Expect a two-line prompt: a context line (path; `exit 1` after `false`) and a
stable `>` input line. In a Git checkout you should also see `git:branch` and
optional `+N` / `~N` / `?N` counts.

```bash
MBX_PRODUCTION_CONTEXT=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
```

Expect a prominent production marker (never color-only). `MBX_DISABLE_GIT=1`
omits Git discovery. `MBX_DISABLE_RENDERER=1` uses the Bash-only fallback; the
shell stays usable if the helper is missing.

### 2. Duration (opt-in)

Only enable this when no `DEBUG` trap is already installed. Default install
never installs `DEBUG`.

```bash
MBX_ENABLE_DURATION_TIMING=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
sleep 3
```

Expect elapsed time on the next prompt (shown at ≥ 2 s). `trap -p DEBUG` after
a default (timing-off) install must match the pre-source trap.

### 3. History sidecar (opt-in)

```bash
MBX_HISTORY=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
echo hello-mbx
"$MBX_BIN" history count
"$MBX_BIN" history search recent --limit 5
"$MBX_BIN" history search prefix echo --limit 5
"$MBX_BIN" history search cwd "$PWD" --limit 5
"$MBX_BIN" history search fuzzy git --limit 5
"$MBX_BIN" history search failed --limit 5
"$MBX_BIN" history path
```

The first prompt after source is not recorded. Later admitted commands appear
in search. `.bash_history` is not rewritten. `MBX_HISTORY_EXCLUDE='echo *'`
drops matching commands. `history path|count|clear|delete` also work as
`"$MBX_BIN" history …`. Stored command text is plaintext local data.

### 4. Insert token (`bind -x`)

Default chord is `Ctrl-X` then `Ctrl-Y`. The default token is a test sentinel
(`printf 'MBX_EDT:ok\n'`). For a safer try, insert a word instead:

```bash
MBX_EDITOR_INSERT_TOKEN=hello bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
```

Type `echo `, press `Ctrl-X Ctrl-Y`, then type more text. The token appears at
the cursor and is **not** run until you press Enter. If that chord is already
bound, MBX leaves it alone unless `MBX_EDITOR_OVERRIDE=1`.

### 5. Stock Tab completion

Default install does **not** wrap `ls` or `printf`. Create a unique file and
Tab-complete it:

```bash
touch MBX_COMP_UNIQUE
printf 'GOT:%s|\n' MBX_COMP_U
```

Press Tab, then Enter. Expect `GOT:MBX_COMP_UNIQUE|` (stock Bash quoting). There
is no completion popup.

### 6. Wrapped `-F` metadata (developer)

This is an adapter harness, not a menu. Wrap one existing `-F` completer, Tab as
usual, then inspect parallel arrays. Insertion bytes stay stock.

```bash
# After source bash/init.bash in an interactive shell:
_mbx_comp_wrap_existing_f git   # skip if git has no -F spec
git sta
```

Press Tab. The line should match stock Git completion. Then:

```bash
printf 'kinds=%s scores=%s order=%s\n' \
  "${#_MBX_COMP_KINDS[@]}" "${#_MBX_COMP_SCORES[@]}" "${#_MBX_COMP_ORDER[@]}"
```

Do not set `MBX_COMP_FIXTURES=1` in a daily shell; that flag is for automated
tests only.

### 7. Ranked-accept chord (`bind -x`)

After Tab on a wrapped `-F` completion, MBX records `_MBX_COMP_RANKED_REPLY`
(top of `_MBX_COMP_ORDER`). Default chord is `Ctrl-X` then `Ctrl-A`. It replaces
the current word with that candidate when the word is a prefix of it, and does
not execute the text. Tab insertion bytes stay stock.

```bash
# After source bash/init.bash in an interactive shell:
_mbx_comp_wrap_existing_f git   # skip if git has no -F spec
git sta
```

Press Tab (stock insertion), then `Ctrl-X Ctrl-A` to replace the current word
with the top-ranked candidate. If the chord is already bound, MBX leaves it
alone unless `MBX_COMP_ACCEPT_OVERRIDE=1`.

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
MBX_EDITOR_INSERT_TOKEN=hello   # text inserted by Ctrl-X Ctrl-Y
MBX_EDITOR_INSERT_KEYSEQ='\C-x\C-y'
MBX_EDITOR_OVERRIDE=1           # overwrite an occupied insert chord
MBX_COMP_ACCEPT_KEYSEQ='\C-x\C-a'  # ranked-accept chord (default)
MBX_COMP_ACCEPT_OVERRIDE=1      # overwrite an occupied ranked-accept chord
MBX_LOG=trace                   # helper timing/events; never logs command text
```

`NO_COLOR` and `TERM=dumb` are respected. If the helper is missing or exits, the
shell continues with a Bash-only prompt.

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
