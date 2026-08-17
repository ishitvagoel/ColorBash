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
2. When enabled, emacs `self-insert` for a bounded ASCII set (letters, digits,
   space, and `_ - . : / = + , [ ] @ ~`) is wrapped with `bind -x`. Stock
   `self-insert` is replaceable. A user or `-x` binding that is not
   `self-insert` is skipped unless `MBX_GHOST_OVERRIDE=1`.
3. The typed prefix stays left of `READLINE_POINT`. A matching sidecar prefix
   row may extend `READLINE_LINE` to the right of the cursor. That suffix is
   not accepted until the user moves point to the end (default Right /
   `\C-f`).
4. Enter (`\C-m`) runs an internal strip chord (default `\C-xg`, unbound in
   stock emacs; M-040) then stock `\C-j` `accept-line`. Unaccepted suffix is
   discarded. Suggestions never execute automatically. `\C-j` stays
   `accept-line`.
5. Helper timeout, missing binary, or a match that is not an exact byte prefix
   / contains controls leaves the typed character inserted and no suffix.
   Query budget is `MBX_GHOST_TIMEOUT` (default `MBX_HISTORY_TIMEOUT` / 0.10 s).
6. Dim styling, vi-insert, remaining printables, word-accept, cycling, async
   generation IDs, and highlighting remain later leftovers. Do not steal Tab,
   `\C-r`, `\C-g`, `\C-x\C-r`, or `\C-x\C-s`.

## Alternatives

- Paint dim text after `bind -x` returns: Readline redisplays and erases it.
- Rebind every printable including quotes and arrows: larger keymap risk;
  deferred.
- Custom editor (Strategy B): still unjustified (ADR 0003).
- Default-on ghost: rejected; wrapping `self-insert` must be explicit.

## Consequences

Users can enable fish-style “text after the cursor” from sidecar history
without a decoration hook. The suffix is ordinary command bytes, not styled
ghost. Async non-blocking lookup (`GHST-001`) stays later. Highlighting and
GUI overlays stay blocked.

## Validation

PTY evidence in `crates/pty/tests/ghost.rs` and module contracts in
`tests/bash/modules.bash`. Plan: `docs/ghst-002-inline-ghost-plan.md`.
