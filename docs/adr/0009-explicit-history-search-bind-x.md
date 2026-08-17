# ADR 0009: Explicit history-search `bind -x` is Strategy A

Status: Accepted (2026-08-16)

## Context

Phase 8 (enhanced Ctrl+R) was blocked together with ghost suggestions and live
highlighting because `G3` recorded that Readline has no after-every-key
decoration hook (ADR 0003, B-5). That leftover is real for continuous
decoration. It is the wrong blocker for an explicit search action.

The product brief and roadmap dependency graph already intended enhanced Ctrl+R
before ghost: an explicit insertion action is a lower-risk validation of
sidecar search and exact text insertion. `G2` permits history-driven editor
experiments. `G3` proved non-destructive `bind -x` insert. Ranked-accept
(`\C-x\C-a`) already uses the same class of binding.

A type-to-filter overlay that redraws on every key would still need an
after-every-key hook or printable-key rebinds. Querying the sidecar when the
user presses a dedicated chord does not.

## Decision

1. Explicit user-invoked history search via `bind -x` is Strategy A (ADR 0003).
   It is not continuous decoration and does not require taking editor ownership.
2. The default chord is `\C-xh` (Ctrl-X then `h`; unbound in stock emacs;
   M-040). Do not steal stock `\C-r` reverse-i-search. `\C-x\C-r` is Readline
   `re-read-init-file`. `\C-x\C-s` is terminal XOFF under IXON. Occupied
   keyseqs are skipped unless `MBX_SEARCH_OVERRIDE=1`. `MBX_SEARCH_KEYSEQ` may
   select another sequence.
3. The action requires `MBX_HISTORY=1`. Otherwise it is a no-op. Helper
   startup, timeout, empty output, or failure must leave the line unchanged and
   keep the prompt usable.
4. The query is `READLINE_LINE` unless the chord is repeating on the current
   snapshot entry. An empty line requests newest sidecar rows (`history search
   recent`). A non-empty line tries exact prefix, then fuzzy. Results are
   bounded (`MBX_SEARCH_LIMIT`, default 8, max 16). The selected command text
   replaces the entire `READLINE_LINE`; `READLINE_POINT` moves to the end.
   Repeating the chord cycles the snapshot and wraps. The action never executes
   the inserted text. The snapshot clears at the next prompt.
5. A dedicated restore chord (default `\C-xl`; unbound in stock emacs; M-040)
   writes the pre-search `READLINE_LINE` and `READLINE_POINT` back without
   executing. Occupied restore keyseqs are skipped unless
   `MBX_SEARCH_RESTORE_OVERRIDE=1`. `MBX_SEARCH_RESTORE_KEYSEQ` may select
   another sequence. No snapshot, history off, or helper failure is a no-op.
6. Ghost suggestions, live highlighting, and a GUI completion overlay remain
   blocked on after-every-key decoration / editor ownership. Do not rebind
   printable keys to simulate a search UI.

## Alternatives

- Rebind `\C-r` by default: rejected; it would replace stock reverse-i-search.
- Draw a type-to-filter overlay: still blocked; cycling the bounded snapshot is
  the Strategy A result view (`docs/srch-001-result-view-plan.md`).
- Custom editor (Strategy B): still unjustified (ADR 0003).

## Consequences

Phase 8 can ship an explicit search insert, bounded cycling, and cancel
restoration without reopening editor ownership. Stock Ctrl+R remains Bash
reverse-i-search. A metadata overlay and 100k-row interactive latency stay
later `SRCH-003` leftovers. Command text stays out of traces (`M-023`).

## Validation

PTY evidence in `crates/pty/tests/history_search.rs` and module contracts in
`tests/bash/modules.bash`. Plans: `docs/srch-001-history-search-plan.md`,
`docs/srch-001-result-view-plan.md`, `docs/srch-002-cancel-restore-plan.md`.
