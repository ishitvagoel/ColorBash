# ADR 0010: Opt-in inline ghost via stock self-insert wrapping

Status: Accepted (2026-08-17)

## Context

Phase 4 ghost suggestions need the current buffer after ordinary typing.
Readline has no decoration callback on every key (ADR 0003, B-5). A GUI overlay
and live highlighting still need that hook. Ghost does not have to paint dim
ANSI after Readline redisplays: the suggestion can live in `READLINE_LINE`
*after* `READLINE_POINT`. Readline already draws that suffix to the right of
the cursor.

ADR 0009 keeps explicit search on a dedicated chord and still forbids
rebinding printables to fake a Ctrl+R overlay. That prohibition does not bar a
separate, opt-in ghost path. `G2` already permits history-driven editor
experiments.

## Decision

1. Opt-in inline ghost (`MBX_GHOST=1` and `MBX_HISTORY=1`) is Strategy A.
   Readline remains the redisplay owner. MBX does not take editor ownership and
   does not put ANSI in `READLINE_LINE`.
2. When enabled, `self-insert` for ASCII printables is wrapped with `bind -x`
   on emacs and vi-insert in an interactive tty. Stock `self-insert` is
   replaceable. A user or `-x` binding that is not `self-insert` is skipped
   unless `MBX_GHOST_OVERRIDE=1`. Helper lookup runs with job-control
   monitor/notify off so per-key forks do not print jobs or accept the line.
   Piped `bash -i` is not wrapped (M-042). vi-command is not wrapped.
3. The typed prefix stays left of `READLINE_POINT`. A matching sidecar prefix
   row may extend `READLINE_LINE` to the right of the cursor. That suffix is
   not accepted until the user moves point (Right / `\C-f` for the full row;
   `\ef` / Ctrl-Right `forward-word` for one alphanumeric word). Ctrl-X Ctrl-N
   / Ctrl-X Ctrl-P cycle other exact-prefix rows without accepting them.
   Left / `\C-b` / `\eOD` strip an unaccepted suffix then `backward-char`.
   Home / `\C-a` / CSI Home, Up / `\C-p` / CSI Up, and backward-word strip
   first as well. Kill-ring isolation stays a later leftover.
4. Enter (`\C-m`) and newline (`\C-j`, when it is stock `accept-line`) stay
   `accept-line` except while a suffix is active. Then both are a Readline-only
   macro: reserved kill-line (default `\C-x\C-k`) from point, then reserved
   accept-line (default `\C-x\C-m`). Terminals often deliver Enter as `\n`
   (`\C-j`) via `icrnl`. `bind -x` cannot be chained in a keyseq macro because
   remaining keys are dropped (M-041). Do not `eval` the line. Occupied helper
   chords skip install. Do not use `\C-xg` or a letter suffix that ghost also
   wraps.
5. Helper timeout, missing binary, or a match that is not an exact byte prefix
   / contains controls leaves the typed character inserted and no suffix.
   Query budget is `MBX_GHOST_TIMEOUT` (default `MBX_HISTORY_TIMEOUT` / 0.10 s).
6. Dim styling, async generation IDs, and highlighting remain later leftovers.
   Remaining ASCII printables that are stock `self-insert` are wrapped using
   Readline quoted keyseqs. vi-insert uses the same helpers; `\ef` is not
   bound there because ESC is `vi-movement-mode`. Prefix-match cycling uses
   unbound `\C-x\C-n` / `\C-x\C-p`. Left / `\C-b` dismiss an unaccepted suffix.
   Home / Up / backward-word dismiss before their stock motion. Kill-ring
   isolation remains a later leftover.
   Do not steal Tab, `\C-r`, `\C-g`, `\C-x\C-r`, or `\C-x\C-s`.

## Alternatives

- Paint dim text after `bind -x` returns: Readline redisplays and erases it.
- Rebind vi-command: deferred. Left / backward-char, Home, Up, and
  backward-word are wrapped on emacs and vi-insert where stock motion is
  replaceable. ASCII printables that are stock `self-insert` are wrapped on
  emacs and vi-insert.
- Custom editor (Strategy B): still unjustified (ADR 0003).
- Default-on ghost: rejected; wrapping `self-insert` must be explicit.

## Consequences

Users can enable fish-style “text after the cursor” from sidecar history
without a decoration hook. The suffix is ordinary command bytes, not styled
ghost. Async non-blocking lookup (`GHST-001`) stays later. Highlighting and
GUI overlays stay blocked.

## Validation

PTY evidence in `crates/pty/tests/ghost.rs` and module contracts in
`tests/bash/modules.bash`. Plans: `docs/ghst-002-inline-ghost-plan.md`,
`docs/ghst-002-printables-plan.md`, `docs/ghst-002-vi-insert-plan.md`,
`docs/ghst-002-left-motion-plan.md`, `docs/ghst-002-home-up-motion-plan.md`, `docs/ghst-003-word-accept-plan.md`,
`docs/ghst-003-cycle-plan.md`.
