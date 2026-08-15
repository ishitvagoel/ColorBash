# ADR 0003: Augment Readline before considering a custom editor

Status: Accepted for MVP experiments

## Context

Live highlighting, suggestions, multiline layout, and popup completion need access
to the current editing buffer. Bash exposes programmable completion and `bind -x`,
but not a supported decoration callback on every keypress.

## Decision

Begin with Strategy A: augment Bash/Readline using prompt hooks, programmable
completion adapters, and a small number of configurable `bind -x` actions. Keep
Readline responsible for cursor motion, paste, resize, and redraw. Prototype an
advanced Bash line-editor integration only for gaps proven by user value. Do not
adopt a custom frontend during MVP.

## Alternatives

- An advanced line-editor layer offers richer rendering but increases keymap,
  quoting, paste, signal, and terminal-restoration risk.
- A custom frontend gives maximal control but must reproduce Readline behavior and
  delegate execution without breaking jobs/signals; risk is currently unjustified.

## Consequences

The MVP protects familiar editing semantics and can ship vertical slices. True
continuous syntax decoration and GUI-like popups may be limited or delayed.

## Risks

Rebinding keys may collide with user configuration; completion functions have
hidden state; attempts to simulate highlighting could flicker or corrupt wraps.

## Validation plan

Prototype one suggestion-insertion binding and one standard completion adapter.
Test emacs/vi modes, bracketed paste, multiline input, resize, Ctrl+C/Ctrl+Z, tmux,
SSH, and fullscreen programs. Revisit only with a measured capability gap.

