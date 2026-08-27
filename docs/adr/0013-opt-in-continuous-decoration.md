# ADR 0013: Opt-in continuous decoration via self-insert wrapping

Status: Accepted (2026-08-27)

## Context

Live syntax highlighting and completion overlays were deferred because Readline
exposes no supported after-every-key decoration callback (ADR 0003, B-5). Ghost
suffix (ADR 0010) already proved that wrapping stock `self-insert` with `bind -x`
is Strategy A: Readline remains the redisplay owner, and explicit opt-in is
required.

Product revisit (2026-08-27) authorizes the first continuous-decoration slice
when all of the following hold:

1. plain command bytes remain recoverable before execution;
2. styling uses Readline non-printing markers (`\001` … `\002`), not bare ANSI
   that would corrupt cursor math or execute on Enter;
3. Tab insertion and stock `\C-r` stay untouched;
4. ghost and highlight are not combined until composition is evidenced.

## Decision

1. Opt-in syntax highlighting (`MBX_HIGHLIGHT=1`) wraps stock `self-insert` on
   emacs and vi-insert in an interactive tty, matching ADR 0010 install rules
   (skip occupied bindings unless `MBX_HIGHLIGHT_OVERRIDE=1`; skip piped
   `bash -i`). A parallel plain buffer `_MBX_HIGHLIGHT_PLAIN` and plain cursor
   `_MBX_HIGHLIGHT_POINT` are the execution source of truth.
2. On each wrapped insert, delete, or motion that changes the plain buffer,
   Bash calls `mbx highlight PLAIN --point N`. The helper returns two lines:
   styled `READLINE_LINE` (markers + SGR) and the styled cursor index. Helper
   failure leaves the plain bytes visible with no styling.
3. Accept-line (`\C-m` / `\C-j`) is wrapped while highlighting is active. The
   wrapper restores `READLINE_LINE` and `READLINE_POINT` from the plain buffers,
   then invokes stock `accept-line`. Executed bytes never contain styling.
4. Opt-in completion overlay (`MBX_COMP_OVERLAY=1`) is Strategy A metadata
   display, not Tab replacement. After a wrapped completion populates
   `_MBX_COMP_RANKED_LIST`, `\C-x\C-o` toggles a bounded terminal overlay drawn
   below the prompt with `\e7`/`\e8` (DECSC/DECRC). `\C-xn` / `\C-xp` move the
   selection; `\C-x\C-a` inserts the selected candidate (existing ranked-accept);
   `\C-g` dismisses. Tab stays stock Bash insertion.
5. `MBX_HIGHLIGHT=1` and `MBX_GHOST=1` together are unsupported in this slice;
   highlight install skips when ghost is enabled.
6. Dim ghost paint (ANSI after redisplay) remains out of scope. This ADR covers
   plain-buffer highlighting and a completion candidate overlay only.

## Alternatives

- Paint ANSI after `bind -x` without markers: Readline redisplays and breaks
  cursor math (ADR 0010).
- Rebind Tab for popup navigation: rejected (ADR 0006 / COMP-004 policy).
- Custom editor (Strategy B): still unjustified.

## Consequences

Phase 6 highlighting and `COMP-004` overlay move from indefinite deferral to
bounded Strategy A implementations. Type-to-filter Ctrl+R overlays and GUI
menus remain deferred. Latency percentiles stay deferred.

## Validation

Plans: `docs/hlt-001-lexer-plan.md`, `docs/hlt-002-integration-plan.md`,
`docs/comp-004-overlay-plan.md`, review leftovers
`docs/hlt-comp-review-close-plan.md`. Module contracts in `tests/bash/modules.bash`;
PTY cases in `crates/pty/tests/highlight.rs` and
`crates/pty/tests/completion_harness.rs` (overlay cases). Do not treat H-1–H-6
as satisfied until `bind -X` lists highlight widgets.
