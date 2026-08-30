# MBX — Modern Bash Experience

MBX is an experimental interaction layer for **interactive Bash**. Bash still
parses, expands, and runs every command. MBX only enriches the prompt and, when
you opt in, records admitted history and offers insert-only suggestions.

Suggestions and selections insert ordinary Bash text. They **never execute**
until you press Enter.

## Quick start

One command builds the helper and turns on the **comfort** profile (history
search, ghost suggestions, completion overlay). It does **not** edit
`~/.bashrc` unless you pass `--bashrc`:

```bash
bash scripts/install.bash --interactive   # menu: pick every option, then w to save
bash scripts/install.bash --bashrc        # comfort preset + persist in ~/.bashrc
mbx_configure                             # later: same menu, starting from the saved file
mbx_status                                # flags, helper path, duration, persist-bashrc
mbx_doctor                                # diagnose: what's off and how to fix it
```

`--interactive` opens a numbered menu for every user-facing option (features,
prompt, history extras, keys, timeouts). Re-running it (or `mbx_configure`)
starts from the saved file when one exists; choose **4) Current config** to
keep those values. Ghost and syntax highlighting cannot both be on. It does
**not** edit `~/.bashrc` unless you turn on persist (option 15) or pass
`--bashrc`. Non-interactive overlays: `bash scripts/configure.bash
--from-config --answers FILE`. `--build` runs `cargo build --release
--workspace` before writing; the menu defaults to `--no-build`.

Comfort is the highest-QoL preset. History is local SQLite and does not rewrite
`.bash_history`. Use `--profile highlight` for coloring instead of ghost, or
`--profile prompt` for the prompt only. Non-interactive: `bash
scripts/configure.bash --answers FILE`.

Requirements: Bash 5.x, Rust **1.85** or newer, a real terminal (not a pipe).
Git is optional (prompt segment and `MBX_COMP_WRAP=git`). If several
toolchains are installed: `export RUSTUP_TOOLCHAIN=1.85.0`. There is no
prebuilt binary yet — building from source is currently the only install
path (see [What remains](#what-remains)).

Disable without uninstalling by editing `~/.config/mbx/config.bash`, running
`mbx_configure`, or `export MBX_HISTORY=0`. Remove the bashrc block with
`bash scripts/install.bash --uninstall-bashrc`. Delete the source line and
start a new shell to unload MBX. The history store is
`$XDG_DATA_HOME/mbx/` or `~/.local/share/mbx/`.

Environment variables already set in the shell always win over the config
file. `source bash/init.bash` and `scripts/dev-setup.bash` still never write
`~/.bashrc`.

## What to expect in every session

These rules hold whether a feature is on or off:

1. **Bash owns execution.** Exit status, jobs, aliases, functions, traps,
   history, completion, and quoting stay ordinary Bash.
2. **The prompt is two lines.** A context line (path and optional Git, failure,
   SSH, production, duration) plus a stable `>` input line.
3. **Helper failure is not a broken shell.** Missing helper, timeout, or
   malformed output falls back to a usable Bash prompt on that cycle.
4. **Opt-in features stay off until set to `1`.** Unset `MBX_HISTORY`,
   `MBX_GHOST`, `MBX_HIGHLIGHT`, and `MBX_COMP_OVERLAY` do nothing.
5. **Occupied Readline chords are skipped.** MBX does not steal a key unless
   you set the matching `*_OVERRIDE=1`.
6. **Use a tty.** Piped `bash -i` is not PTY evidence and will skip
   self-insert wrapping (ghost and highlight).

## Feature map

| Feature | Enable | Default keys | Executes text? |
| --- | --- | --- | --- |
| Adaptive prompt | `source bash/init.bash` | — | No (display only) |
| Command duration | `MBX_ENABLE_DURATION_TIMING=1` | — | No |
| History sidecar | `MBX_HISTORY=1` | — | No (records after admission) |
| History-search insert | `MBX_HISTORY=1` | `Ctrl-X` `h` insert, `Ctrl-X` `l` restore | No until Enter |
| Ghost suffix | `MBX_HISTORY=1` and `MBX_GHOST=1` | Right accept; Left dismiss; `Ctrl-X Ctrl-N`/`P` cycle | Enter runs **typed prefix only** |
| Insert token | default on | `Ctrl-X Ctrl-Y` | No until Enter |
| Stock Tab | always | Tab | No (stock Bash insert) |
| Ranked accept / cycle | wrap a `-F` completer, then Tab | `Ctrl-X Ctrl-A`; `Ctrl-X` `n`/`p` | No until Enter |
| Completion overlay | `MBX_COMP_OVERLAY=1` after wrapped Tab | `Ctrl-X Ctrl-O` toggle; `Ctrl-X` `j` dismiss | No until Enter |
| Syntax highlighting | `MBX_HIGHLIGHT=1` | self-insert wrap | Enter runs **plain** bytes; live color currently off (`M-064`) |

Incompatible: **`MBX_GHOST=1` and `MBX_HIGHLIGHT=1` together.** Highlight
install skips when ghost is enabled.

Not in this MVP: dim after-every-key ghost paint, type-to-filter Ctrl+R
overlay, macOS PTY matrix, live highlight color (`M-064`). Canonical status:
[`docs/roadmap.md`](docs/roadmap.md).

For a walkthrough and worked example of every row above — what to type, what
to expect, the automated test that covers it, and the full environment
variable reference — see **[`docs/reference.md`](docs/reference.md)**.

## What remains

These MVP leftovers are **not** available for interactive use:

| Feature | Why it is waiting |
| --- | --- |
| Ghost dim / live paint | Opt-in suffix ghost exists (ADR 0010); dim after-every-key styling does not |
| Type-to-filter Ctrl+R overlay | Explicit `\C-xh` insert exists (ADR 0009); redraw-on-key overlay does not |
| Live syntax highlighting | Full pipeline works; Readline renders its own invisible-marker bytes visibly inside the edit buffer, so color stays off (`M-064`, open) |
| Prebuilt binaries | No tag has been cut yet; `.github/workflows/release.yml` exists but is untested (`REL-001`, in progress) |
| macOS PTY matrix | `deferred` (ADR 0012); needs a macOS host. Linux nested/SSH/login/vim/tmux PTY is recorded |

Strategy A MVP on Linux is `complete` (`G5` 2026-08-27). Opt-in highlighting
stays `blocked` on `M-064`; the completion overlay is `validation` since
`M-065` was fixed (it reserves its rows before saving the cursor, so a draw
that scrolls can no longer corrupt the prompt). Dim paint, type-to-filter overlays, and
macOS matrix are **G5 revisit**.

The helper bundles SQLite (`rusqlite` with the `bundled` feature) for the
history store. The protocol crate remains dependency-free. History capture
stays off unless `MBX_HISTORY=1`.

See [`docs/architecture.md`](docs/architecture.md) for dependency direction and
the limits of the current prompt, history, editor, and completion
implementations.

## Documentation

Agent work starts with [`AGENTS.md`](AGENTS.md), which requires reading the
cumulative [`MISTAKES.md`](MISTAKES.md) before planning or editing.

- [`docs/reference.md`](docs/reference.md) — per-feature walkthroughs, full
  environment variable list, and automated test commands
- [`docs/roadmap.md`](docs/roadmap.md) — canonical delivery status, gates, and
  next work (full change-log history in
  [`docs/archive/roadmap-history.md`](docs/archive/roadmap-history.md))
- [`docs/ux-spec.md`](docs/ux-spec.md) — prompt hierarchy and interaction
  principles
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/bash-compatibility.md`](docs/bash-compatibility.md)
- [`docs/protocol.md`](docs/protocol.md) / [`docs/protocol-mbx2.md`](docs/protocol-mbx2.md)
- [`docs/hlt-002-integration-plan.md`](docs/hlt-002-integration-plan.md) /
  [`docs/hlt-003-hostile-gate-plan.md`](docs/hlt-003-hostile-gate-plan.md) /
  [`docs/comp-004-overlay-plan.md`](docs/comp-004-overlay-plan.md)
- [`docs/adr/`](docs/adr/) — especially ADR 0009 (search), 0010 (ghost), 0013
  and 0014 (highlight + overlay)
- [`CODEX_MODERN_BASH_ARCHITECTURE.md`](CODEX_MODERN_BASH_ARCHITECTURE.md) —
  originating product brief and long-term intent
