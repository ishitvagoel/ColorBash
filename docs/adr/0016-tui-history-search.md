# ADR 0016: Modal TUI history search on an explicit chord

Status: Accepted (2026-09-03)

## Context

The Strategy A MVP shipped with every "modern shell" interaction driven by
`Ctrl-X` chords over single-line widgets: the history-search chord replaces
the line with a match and re-presses cycle through a bounded snapshot
(ADR 0009). The deferred wishlist named a "type-to-filter Ctrl+R overlay" —
a live-filtering view — and the roadmap kept it `deferred` because redrawing
on every keystroke inside Readline needs an after-every-key hook the shell
does not expose (ADR 0003 B-5), and rebinding printable keys to fake one is a
stop/reassess condition (ADR 0009 decision 6).

The gap this leaves is product-level: nothing in a default session looks or
feels special, and the single highest-value deferred interaction — fzf/atuin
style full-screen history filtering — appears blocked by the same redisplay
limitation.

That blocker does not actually apply to a **modal** picker. ADR 0009's line
was drawn for in-Readline decoration: "A type-to-filter overlay that redraws
on every key would still need an after-every-key hook or printable-key
rebinds. Querying the sidecar when the user presses a dedicated chord does
not." A full-screen picker launched by the existing chord is the second case
taken to its conclusion: while it runs, it owns the terminal; when it exits,
Readline resumes owning the line and receives only the selected text.

## Decision

1. **`mbx tui search [--seed TEXT]`** — a new subcommand of the existing
   `mbx` binary. Opt-in per session with `MBX_TUI=1` (and the established
   enablement rule: unset or `=0` preserves today's behavior byte-for-byte).
   When enabled, the `Ctrl-X h` chord launches the TUI instead of the
   insert-cycle widget; `Ctrl-X l` restore and every other chord are
   unchanged.
2. **Modal, not embedded.** The TUI enters the alternate screen
   (`\e[?1049h`), hides the cursor, filters the local history database
   interactively (type to filter, arrows to select, Enter to accept, Esc or
   Ctrl-C to cancel), then restores the terminal completely before
   returning. Readline keeps editing and redisplay ownership before and
   after (ADR 0003 unchanged). No printable key is rebound (ADR 0009
   decision 6 unchanged); no in-prompt decoration is added.
3. **Data path is in-process.** The TUI reads the same SQLite store through
   the existing `HistorySearch` port, mirroring the chord widget's tiers
   (typed prefix — cwd-scoped first, then global — then fuzzy over the
   bounded recent pool). No MBX2 wire frames, no coprocess involvement; the
   hot-path coprocess budget is untouched. A user-invoked interactive
   command is the `USER_COMMAND_BUSY_DEADLINE_MS` case, not the 100 ms hot
   path.
4. **Zero new dependencies.** Terminal handling is hand-rolled FFI matching
   the house style of `crates/pty/src/sys.rs`: `tcgetattr`/`tcsetattr` for
   raw mode, `TIOCGWINSZ` for size, `poll`/`read` for input, and a
   `sigaction`-based SIGWINCH flag for resize. Key decoding, the renderer,
   and width math (`unicode-width`, already a dependency) are in-crate.
5. **Terminal safety is the exit criterion, not polish.** On every exit
   path — selection, cancel, error, panic — the TUI leaves the terminal
   exactly as it found it (`stty -g` identical, alternate screen exited,
   cursor visible). PTY evidence must prove: restore on select, cancel,
   Ctrl-C, resize mid-session, and helper failure; insertion of the exact
   selected bytes with no execution; refusal of hostile rows by the
   existing C0/DEL gate; and fallback to the chord widget when the helper
   is missing.
6. **Bash side stays a sibling of the chord widget.** `_mbx_search_insert`
   spawns the helper with the established recipe — `_mbx_jobs_suspend`
   around the child, exit-status gating, one selected line read back,
   `_mbx_text_has_c0_or_del` before any `READLINE_LINE` assignment — then
   snapshots the original line for `Ctrl-X l`. A failed or missing helper
   falls through to today's query path. Nothing executes until the user
   presses Enter, exactly as before.

## Consequences

- The roadmap's "type-to-filter Ctrl+R overlay stays `deferred`" line is
  superseded **for history search only**; the completion-picker TUI and any
  in-prompt live decoration remain `deferred` with their original reasons.
- The no-heavy-dependency rule holds: the workspace gains no crates. The
  cost is owned key-decoding and rendering code (~600–900 lines), confined
  to two new modules with their own tests.
- Raw-mode input is a new risk class for the shell integration. The `stty
  -g`-before/after oracle already used for fullscreen programs
  (`vim_fullscreen_quits_and_restores_stty`) becomes the TUI's gate; a
  restore-on-panic hook is mandatory, not optional polish.
- SSH and tmux inherit whatever the terminal emulator does with the
  alternate screen; the PTY matrix cases already cover vim's alternate
  screen, and the TUI rides the same mechanism.
- Latency inside the picker (per-keystroke query cost) is recorded, not
  gated, per the accepted latency-deferral policy.

## Alternatives considered

- **crossterm/ratatui**: battle-tested, but the project adds no dependency
  without measured need, this would be its first major one, and Linux-only
  MVP needs only four syscalls. Rejected for now; an ADR can revisit if the
  hand-rolled layer proves costly.
- **In-Readline type-to-filter overlay**: the original deferred idea;
  requires the after-every-key hook that does not exist. Rejected — this is
  exactly the wall ADR 0009 documented.
- **Stock Ctrl-R takeover**: desirable ergonomics, but Ctrl-R is stock Bash;
  the override machinery could permit it later. Out of scope here.
