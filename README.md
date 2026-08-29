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
toolchains are installed: `export RUSTUP_TOOLCHAIN=1.85.0`.

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
| Syntax highlighting | `MBX_HIGHLIGHT=1` | self-insert wrap | Enter runs **plain** bytes; live color currently off (§10, `M-064`) |

Incompatible: **`MBX_GHOST=1` and `MBX_HIGHLIGHT=1` together.** Highlight
install skips when ghost is enabled.

Not in this MVP: dim after-every-key ghost paint, type-to-filter Ctrl+R
overlay, macOS PTY matrix, live highlight color (`M-064`). Canonical status:
[`docs/roadmap.md`](docs/roadmap.md).

---

## Using and testing each feature

Run each scenario in a real terminal after `source bash/init.bash`. None of
these execute a suggestion on your behalf.

### 1. Adaptive prompt

**Enable:** source the loader. No extra flag.

**What you should see:**

```text
~/projects/api  git:main ~2 ?1  exit 1
>
```

| Segment | When it appears | Meaning |
| --- | --- | --- |
| path | always | Compact cwd; `~` for `$HOME` |
| `git:branch` | Git worktree, Git not disabled | Current branch or detached fallback |
| `+N` | staged changes | Index count |
| `~N` | unstaged changes | Worktree count |
| `?N` | untracked files | Untracked count |
| `exit N` | previous command failed | Exact status, never color-only |
| `ssh:host` | SSH session, not production | Host label |
| `! PROD · host · user` | `MBX_PRODUCTION_CONTEXT=1` | Replaces SSH; never color-only |
| duration | opt-in timing, ≥ 2 s | Elapsed time of last command |

**Try:**

```bash
cd ~
false
```

Expect `exit 1` on the next context line and a usable `>` prompt.

```bash
MBX_PRODUCTION_CONTEXT=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
```

Expect a prominent `! PROD` marker. `MBX_DISABLE_GIT=1` omits Git discovery.
`MBX_DISABLE_RENDERER=1` uses the Bash-only fallback. `MBX_COLOR=never`,
`NO_COLOR`, and `TERM=dumb` render plain text.

**Check it is installed:**

```bash
[[ -n $MBX_BIN && -x $MBX_BIN ]] && echo "helper=$MBX_BIN"
"$MBX_BIN" handshake
```

**Automated:** `bash tests/bash/modules.bash`, `cargo test -p mbx-pty --test foundation`.

### 2. Command duration (opt-in)

**Enable only when no `DEBUG` trap is already installed.** Default install
never installs `DEBUG`.

```bash
MBX_ENABLE_DURATION_TIMING=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
sleep 3
```

**Expect:** elapsed time on the next prompt (shown at ≥ 2 s). After a default
(timing-off) install, `trap -p DEBUG` must match the pre-source trap.

### 3. History sidecar (opt-in)

```bash
MBX_HISTORY=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
echo hello-mbx
"$MBX_BIN" history count
"$MBX_BIN" history path
"$MBX_BIN" history search recent --limit 5
"$MBX_BIN" history search prefix echo --cwd "$PWD" --limit 5
"$MBX_BIN" history search prefix echo --limit 5
"$MBX_BIN" history search cwd "$PWD" --limit 5
"$MBX_BIN" history search fuzzy git --limit 5
"$MBX_BIN" history search failed --limit 5
"$MBX_BIN" history search repo "$PWD" --limit 5
"$MBX_BIN" history search branch "$(git rev-parse --abbrev-ref HEAD 2>/dev/null)" --limit 5
```

**Expect:**

- The first prompt after `source` is **not** recorded.
- Later **admitted** commands appear in search (same rules as Bash history:
  `ignorespace`, `ignoredups`, `HISTIGNORE`, `set +o history` still apply).
- `.bash_history` is not rewritten.
- `MBX_HISTORY_EXCLUDE='echo *'` drops matching commands (colon-separated globs).
- In a Git worktree, rows store repository root and branch for `search repo` /
  `search branch`.
- Stored command text is plaintext **local** data. It never goes to telemetry.

`"$MBX_BIN" history clear` empties rows; `"$MBX_BIN" history delete` removes
the SQLite files (`-wal`/`-shm` included). `"$MBX_BIN" history path` prints the
store location.

**Automated:** `cargo test -p mbx-pty --test history_recording`,
`--test history_invariance`, `--test history_admission`.

### 4. History-search chord (`Ctrl-X` `h`)

Requires `MBX_HISTORY=1`. Stock `Ctrl-R` reverse-i-search and
`Ctrl-X Ctrl-R` re-read-init-file stay unchanged.

The chord **replaces the whole line** with a sidecar match (exact prefix, then
fuzzy). Empty and typed queries prefer `$PWD`, then newest rows. It does **not**
run the match. Press the chord again to cycle a bounded snapshot (default 8,
max 16). `Ctrl-X` then `l` restores the typed line.

```bash
MBX_HISTORY=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
printf 'MBX_SRCH:alpha\n'
printf 'MBX_SRCH:beta\n'
```

At the next prompt type `printf 'MBX_SRCH:a` and press `Ctrl-X` then `h`, then
Enter. Expect `MBX_SRCH:alpha`. An empty line plus the same chord inserts the
newest row from `$PWD` (or the global newest if this directory has no rows).
`MBX_SEARCH_FAILED=1` prefers failed empty-line rows first.
`MBX_SEARCH_CWD=0` uses global recent only on an empty line.

If the chord is already bound, MBX leaves it alone unless
`MBX_SEARCH_OVERRIDE=1` (insert) or `MBX_SEARCH_RESTORE_OVERRIDE=1` (restore).

**Automated:** `cargo test -p mbx-pty --test history_search`.

### 5. History ghost suffix (opt-in)

Requires `MBX_HISTORY=1` **and** `MBX_GHOST=1`. Do not combine with
`MBX_HIGHLIGHT=1`. Needs a tty.

```bash
MBX_HISTORY=1 MBX_GHOST=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
echo unique-ghost-alpha
```

Type `echo unique-ghost-a` and pause. The rest of the previous command should
appear **after the cursor** as ordinary command text (not dim paint).

| Key | Effect |
| --- | --- |
| Enter | Runs **only what you typed** (suffix discarded) |
| Right Arrow | Accepts the full suggestion into the line |
| Left / Home / Ctrl-Left | Dismisses the suffix, then stock motion |
| Up / Down / Ctrl-P | Dismiss, then stock history motion |
| Ctrl-N after Ctrl-P | Restores the remembered typed prefix |
| Alt-F / Ctrl-Right (emacs) | Accept one word |
| Ctrl-Right (vi-insert) | Accept one word |
| `Ctrl-X Ctrl-N` / `Ctrl-X Ctrl-P` | Cycle other prefix matches |

**Check:** `_MBX_GHOST_BOUND` should be `1` after install.

**Automated:** `cargo test -p mbx-pty --test ghost`.

### 6. Insert token (`Ctrl-X Ctrl-Y`)

Default token is a test sentinel (`printf 'MBX_EDT:ok\n'`). For a safer try:

```bash
MBX_EDITOR_INSERT_TOKEN=hello bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
```

Type `echo `, press `Ctrl-X Ctrl-Y`, then type more text. The token appears at
the cursor and is **not** run until Enter. Tokens with C0/DEL bytes are refused.
If that chord is already bound, MBX leaves it alone unless
`MBX_EDITOR_OVERRIDE=1`.

**Automated:** `cargo test -p mbx-pty --test editor_bind_x`.

### 7. Stock Tab completion

Default install does **not** wrap `ls` or `printf`. Tab stays stock Bash.

```bash
touch MBX_COMP_UNIQUE
printf 'GOT:%s|\n' MBX_COMP_U
```

Press Tab, then Enter. Expect `GOT:MBX_COMP_UNIQUE|` (stock Bash quoting). There
is no popup unless you enable the overlay (section 9) **and** wrap a `-F`
completer.

**Automated:** `bash tests/bash/smoke.bash`, completion cases in
`crates/pty/tests/completion_harness.rs`.

### 8. Wrapped `-F` completion, ranked accept, and cycle

This is an adapter around an existing Bash `-F` completer, not a replacement
for Tab. The comfort install sets `MBX_COMP_WRAP=git`. You can also wrap
manually:

```bash
# After source bash/init.bash in an interactive shell:
_mbx_comp_wrap_existing_f git   # skip if git has no -F spec
git sta
```

Press Tab. Insertion bytes should match stock Git completion.

Then:

- `Ctrl-X Ctrl-A` replaces the **current word** with the top-ranked candidate
  when that word is still a prefix of it (and still at the Tab snapshot
  offset). It does not execute.
- `Ctrl-X` then `n` / `p` rotate next / previous ranked candidates once the
  current word equals the ranked reply.
- The snapshot clears at the next prompt. A later unrelated word is left
  unchanged.

Inspect metadata after Tab (developer):

```bash
printf 'kinds=%s scores=%s order=%s ranked=%s\n' \
  "${#_MBX_COMP_KINDS[@]}" "${#_MBX_COMP_SCORES[@]}" \
  "${#_MBX_COMP_ORDER[@]}" "${_MBX_COMP_RANKED_REPLY-}"
```

Do **not** set `MBX_COMP_FIXTURES=1` in a daily shell; that flag defines test
commands only.

Occupied chords are skipped unless `MBX_COMP_ACCEPT_OVERRIDE=1` or
`MBX_COMP_CYCLE_OVERRIDE=1`.

**Automated:** `cargo test -p mbx-pty --test completion_harness`.

### 9. Completion overlay (opt-in)

Requires `MBX_COMP_OVERLAY=1` **and** a wrapped `-F` completer (section 8).
Tab itself stays stock; the overlay is a metadata list below the prompt.

```bash
MBX_COMP_OVERLAY=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
_mbx_comp_wrap_existing_f git
git sta
```

Press Tab (stock insert), then `Ctrl-X Ctrl-O`.

**Expect:** up to **eight** ranked rows drawn below the prompt. The selected
row is bold with a `>` marker; optional kind/description appear in muted
text. Display bytes are sanitized (no raw controls).

| Key | Effect |
| --- | --- |
| `Ctrl-X Ctrl-O` | Toggle the list (show / hide) |
| `Ctrl-X` `n` / `p` | Move selection while visible (does not steal ghost cycle keys) |
| `Ctrl-X Ctrl-A` | Insert the **selected** candidate (ranked-accept) |
| `Ctrl-X` `j` | Dismiss and clear |
| `Ctrl-G` | Unchanged stock `abort` |

**Check:** `_MBX_COMP_OVERLAY_BOUND` is `1` and `bind -X` lists
`_mbx_comp_overlay_toggle`. `bind -p` still shows `"\C-g": abort`.

**Automated:** `bash tests/bash/modules.bash` (O-1–O-5),
`cargo test -p mbx-pty --test completion_harness overlay_lists ranked_accept_works_with_overlay_env`.

### 10. Syntax highlighting (opt-in)

Requires `MBX_HIGHLIGHT=1`. Needs a tty. **Skipped** when `MBX_GHOST=1`.

**Live color is currently off by design, not a configuration step you're
missing.** Bash's own Readline renders the `\001`/`\002` markers this feature
uses to make color invisible-width by displaying them as literal `^A`/`^[`
control-character sequences instead of hiding them — that convention only
applies inside `PS1`, not inside the edit buffer (`READLINE_LINE`). Typing at
the prompt with `MBX_HIGHLIGHT=1` exercises the full pipeline below (lexer,
coprocess round trip, exact-byte recovery on Enter) with styling forced off
(`M-064` in `MISTAKES.md`; `docs/adr/0014-highlight-over-coprocess.md`).

```bash
MBX_HIGHLIGHT=1 bash --noprofile --norc
source /absolute/path/to/ColorBash/bash/init.bash
```

Type `if echo "$HOME"; then true; fi # note`. The line still round-trips
through the lexer and redraws correctly; it will not visibly change color
until `M-064` is resolved. Incomplete quotes are still classified (tolerant
lexer). Lines over 4 KiB or containing NUL stay unstyled.

The lexer's token/color mapping below is real and already used by the
standalone `mbx highlight` command (which writes straight to your terminal,
not through Readline, so `M-064` does not apply there):

| Color (16-color SGR) | Token |
| --- | --- |
| bold blue | keywords (`if`, `then`, `export`, …) |
| green | quoted / backtick strings |
| yellow | `$var`, `${…}`, `$(…)` |
| magenta | operators |
| cyan | numbers |
| gray | comments (`#` to end of line) |

**Enter runs the plain command**, not the styled buffer. Motion (Left / Right /
Home) dismisses styling, then moves the cursor on the plain bytes. Helper
failure leaves the line unstyled and usable.

**Check:**

```bash
printf 'bound=%s\n' "${_MBX_HIGHLIGHT_BOUND-}"
bind -X | grep _mbx_highlight_self_insert
```

Both should succeed (`bound=1` and a widget listed). If wrap could not arm
Enter, `_MBX_HIGHLIGHT_BOUND` stays `0` and highlighting is a no-op.

C0 control bytes are not inserted. You can also exercise the helper without a
shell:

```bash
"$MBX_BIN" highlight 'if true; then echo "$HOME"; fi # c' --point 0
"$MBX_BIN" highlight 'ls /tmp/中文/café' --no-color
```

`--no-color` (or a non-tty, with no `--color` override) returns the exact
input bytes. `--color 0|1` overrides that default explicitly; it is what
Bash itself passes for both the coprocess and CLI-fallback interactive paths
(currently always `0`, per `M-064` above).

**Automated:** `cargo test -p mbx highlight::`,
`cargo test -p mbx-pty --test highlight`, highlight contracts in
`bash tests/bash/modules.bash`.

---

## Combining features

| Combination | Result |
| --- | --- |
| Prompt + any opt-in | Supported |
| `MBX_HISTORY=1` + search chord | Supported |
| `MBX_HISTORY=1` + `MBX_GHOST=1` | Supported |
| `MBX_HIGHLIGHT=1` + overlay | Supported (independent) |
| Overlay + ranked accept/cycle | Overlay uses the same snapshot |
| `MBX_GHOST=1` + `MBX_HIGHLIGHT=1` | **Unsupported**; highlight does not install |
| Overlay + stock Tab without wrap | Overlay has nothing to show |

Ghost cycle (`Ctrl-X Ctrl-N` / `Ctrl-X Ctrl-P`) and completion cycle
(`Ctrl-X` `n` / `Ctrl-X` `p`) are different chords on purpose.

## Prototype controls

Group by feature. Unset means off for opt-in flags that require `=1`.

```bash
# User config (~/.config/mbx/config.bash); env already set in the shell wins
# Interactive: bash scripts/configure.bash   or   mbx_configure
# Re-run loads the saved file; overlay with --from-config --answers FILE
MBX_CONFIG=/absolute/path/to/config.bash
MBX_COMP_WRAP=git                # colon-separated -F commands to wrap (comfort default)

# Prompt
MBX_COLOR=never                 # force plain text
MBX_ICONS=never                 # text fallbacks (default auto is also font-safe)
MBX_ICONS=nerd                  # opt in to Nerd Font glyphs
MBX_DISABLE_GIT=1               # omit Git discovery
MBX_DISABLE_RENDERER=1          # use the Bash-only fallback
MBX_IPC_MODE=coprocess          # auto | coprocess | per-call | off
MBX_RENDER_TIMEOUT=0.10         # total native/fallback attempt budget in seconds
MBX_PRODUCTION_CONTEXT=1        # show the prominent production state
MBX_ENABLE_DURATION_TIMING=1    # opt in only when no DEBUG trap is already used

# History sidecar
MBX_HISTORY=1                   # opt in to the local history sidecar
MBX_HISTORY_EXCLUDE='git *'     # colon-separated glob exclusions

# Ghost suffix (needs MBX_HISTORY=1; incompatible with MBX_HIGHLIGHT=1)
MBX_GHOST=1
MBX_GHOST_OVERRIDE=1            # overwrite occupied ghost self-insert keys
MBX_GHOST_LIMIT=8               # max prefix matches collected for cycling (1-8)
MBX_GHOST_DELETE_KEYSEQ='\C-x\C-d' # delete-char helper used by Enter while a suffix is shown
MBX_GHOST_ACCEPT_KEYSEQ='\C-x\C-m' # accept-line helper used by that Enter macro
MBX_GHOST_NEXT_KEYSEQ='\C-x\C-n'   # cycle to the next prefix match
MBX_GHOST_PREV_KEYSEQ='\C-x\C-p'   # cycle to the previous prefix match

# History-search chord
MBX_SEARCH_KEYSEQ='\C-xh'
MBX_SEARCH_OVERRIDE=1
MBX_SEARCH_RESTORE_KEYSEQ='\C-xl'
MBX_SEARCH_RESTORE_OVERRIDE=1
MBX_SEARCH_TIMEOUT=0.10
MBX_SEARCH_LIMIT=8              # bounded snapshot size (max 16)
MBX_SEARCH_CWD=0                # empty-line search uses global recent only
MBX_SEARCH_FAILED=1             # empty-line search prefers failed rows first

# Editor insert token
MBX_EDITOR_INSERT_TOKEN=hello
MBX_EDITOR_INSERT_KEYSEQ='\C-x\C-y'
MBX_EDITOR_OVERRIDE=1

# Ranked completion
MBX_COMP_ACCEPT_KEYSEQ='\C-x\C-a'
MBX_COMP_ACCEPT_OVERRIDE=1
MBX_COMP_CYCLE_NEXT_KEYSEQ='\C-xn'
MBX_COMP_CYCLE_PREV_KEYSEQ='\C-xp'
MBX_COMP_CYCLE_OVERRIDE=1

# Completion overlay (needs a wrapped -F completer)
MBX_COMP_OVERLAY=1
MBX_COMP_OVERLAY_KEYSEQ='\C-x\C-o'
MBX_COMP_OVERLAY_OVERRIDE=1
MBX_COMP_OVERLAY_DISMISS_KEYSEQ='\C-xj'

# Syntax highlighting (incompatible with MBX_GHOST=1)
MBX_HIGHLIGHT=1
MBX_HIGHLIGHT_OVERRIDE=1
MBX_HIGHLIGHT_TIMEOUT=0.05
MBX_HIGHLIGHT_ACCEPT_KEYSEQ='\C-x\C-m'

# Diagnostics (never logs command text)
MBX_LOG=trace
```

`NO_COLOR` and `TERM=dumb` are respected. If the helper is missing or exits, the
shell continues with a Bash-only prompt.

## Automated tests

Canonical suite (format, build, unit tests, Clippy with warnings denied, Bash
syntax, module contracts, protocol integration, compatibility smoke):

```bash
# Required on hosts whose default cargo is older than 1.85:
export RUSTUP_TOOLCHAIN=1.85.0
bash tests/run.bash
```

Focused checks while developing a feature:

```bash
bash tests/bash/modules.bash
bash tests/integration/protocol.bash target/debug/mbx
bash tests/bash/smoke.bash target/debug/mbx

cargo test -p mbx highlight::
cargo test -p mbx-pty --test foundation
cargo test -p mbx-pty --test highlight
cargo test -p mbx-pty --test ghost
cargo test -p mbx-pty --test completion_harness
cargo test -p mbx-pty --test history_search
cargo test -p mbx-pty --test history_recording
cargo test -p mbx-pty --test editor_bind_x
```

A piped interactive Bash process is **not** PTY evidence. Terminal interaction
claims require the PTY harness under `crates/pty/tests/`.

Optional latency (release helper; percentiles are `deferred` and do not gate
product work):

```bash
cargo build --release --workspace
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-prompt.bash target/release/mbx
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-ipc.bash target/release/mbx
```

The IPC benchmark needs permission to create a local Unix-domain socket.

## What remains

These MVP leftovers are **not** available for interactive use:

| Feature | Why it is waiting |
| --- | --- |
| Ghost dim / live paint | Opt-in suffix ghost exists (ADR 0010); dim after-every-key styling does not |
| Type-to-filter Ctrl+R overlay | Explicit `\C-xh` insert exists (ADR 0009); redraw-on-key overlay does not |
| macOS PTY matrix | `deferred` (ADR 0012); needs a macOS host. Linux nested/SSH/login/vim/tmux PTY is recorded |

Strategy A MVP on Linux is `complete` (`G5` 2026-08-27). Opt-in highlighting and
completion overlay are ADR 0013 and remain `validation` until remaining
`HLT-003` latency leftovers are settled. Dim paint, type-to-filter overlays,
and macOS matrix are **G5 revisit**.

The helper bundles SQLite (`rusqlite` with the `bundled` feature) for the
history store. The protocol crate remains dependency-free. History capture
stays off unless `MBX_HISTORY=1`.

See [`docs/architecture.md`](docs/architecture.md) for dependency direction and
the limits of the current prompt, history, editor, and completion
implementations.

## Documentation

Agent work starts with [`AGENTS.md`](AGENTS.md), which requires reading the
cumulative [`MISTAKES.md`](MISTAKES.md) before planning or editing.

- [`docs/roadmap.md`](docs/roadmap.md) — canonical delivery status, gates, and
  next work
- [`docs/ux-spec.md`](docs/ux-spec.md) — prompt hierarchy and interaction
  principles
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/bash-compatibility.md`](docs/bash-compatibility.md)
- [`docs/protocol.md`](docs/protocol.md) / [`docs/protocol-mbx2.md`](docs/protocol-mbx2.md)
- [`docs/hlt-002-integration-plan.md`](docs/hlt-002-integration-plan.md) /
  [`docs/hlt-003-hostile-gate-plan.md`](docs/hlt-003-hostile-gate-plan.md) /
  [`docs/comp-004-overlay-plan.md`](docs/comp-004-overlay-plan.md)
- [`docs/adr/`](docs/adr/) — especially ADR 0009 (search), 0010 (ghost), 0013
  (highlight + overlay)
- [`CODEX_MODERN_BASH_ARCHITECTURE.md`](CODEX_MODERN_BASH_ARCHITECTURE.md) —
  originating product brief and long-term intent
