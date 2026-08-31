# ADR 0014: Route opt-in highlighting through the coprocess

Status: Accepted (2026-08-29)

## Context

`bash/highlight.bash`'s `_mbx_highlight_refresh` forked one `mbx` process per
wrapped keystroke via process substitution
(`exec {fd}< <(exec "$MBX_BIN" highlight ...)`), even when a warm coprocess
was already attached for prompt rendering. Measured on a representative host,
that spawn cost roughly 2.2 ms per invocation before any process-substitution
or `bind -x` overhead, directly on the self-insert path — in conflict with
this repository's own editing budget: "no external command on a cache-hit
keystroke."

Ghost (ADR 0010/0011) already solved the equivalent problem for history
suggestions: an MBX2 `QUERY`/`RESULT` pair with a client-chosen `generation`
that lets Bash skip a stale reply from an in-flight request whose typed
prefix has since changed. Highlighting has the identical hazard — a keystroke
can outrun a prior request's reply on the same coprocess — so the fix is to
extend the same discipline to a new frame pair rather than invent a second
mechanism.

While implementing this, fixing highlighting's color detection (it decided
color from `mbx`'s *own* stdout tty-ness, which is never a terminal on either
the process-substitution or the coprocess path, so color was silently always
off in real use) surfaced a second, independent, more fundamental problem:
Bash's Readline renders `\001`/`\002` (`RL_PROMPT_START_IGNORE`/`_END_IGNORE`)
as visible caret-notation control characters (e.g. `^A`, `^[`) when they
appear inside `READLINE_LINE`, not as the zero-width markers they are inside
`PS1`. Enabling real color in the interactive path with today's marker
technique replaces typed text with garbled output on every keystroke. That
defect (tracked as `M-064`) is out of scope for this ADR: it needs its own
research into a rendering technique Readline actually treats as invisible
within the edit buffer, not a transport change. This ADR's frame carries a
`color` field for exactly that reason — so a future fix only has to flip the
decision Bash sends, not touch the wire format again.

## Decision

1. Add MBX2 `HIGHLIGHT` / `STYLED`, sharing MBX2's magic, framing, and 64-KiB
   bound with `RECORD`/`QUERY`/`RESULT`:
   ```text
   MBX2<TAB>request-id<TAB>HIGHLIGHT<TAB>generation<TAB>color<TAB>point<TAB>text
   MBX2<TAB>request-id<TAB>STYLED<TAB>generation<TAB>point<TAB>line
   ```
   `generation` is a client-chosen decimal u64, echoed on the reply, with the
   same stale-generation-skip contract as ADR 0011: an older generation is
   silently dropped, a newer one is a hard failure for that request.
2. `HIGHLIGHT` is dispatched by a new, independent `HighlightHandler` port
   (`crates/cli/src/highlight_service.rs`), not folded into `HistoryHandler`.
   Highlighting has no privacy or storage contract and `MBX_HIGHLIGHT=1` does
   not require `MBX_HISTORY=1`; gating it behind the history handler would
   make it depend on an unrelated opt-in for no reason.
   `transport::handle_mbx2_line` peeks the frame kind and routes to whichever
   optional handler owns it — `HIGHLIGHT` never reaches `HistoryHandler`, and
   every other kind is unaffected by whether a `HighlightHandler` is present.
3. `HighlightService` is always constructed at the composition root when
   serving MBX2 (it is a pure, storage-free transform); Bash alone decides
   whether to ever send a `HIGHLIGHT` frame, gated on `MBX_HIGHLIGHT=1` as
   before.
4. `bash/highlight.bash` gains `_mbx_highlight_refresh_wire` (coprocess) and
   `_mbx_highlight_refresh_cli` (process-substitution spawn, kept as the
   fallback for `MBX_IPC_MODE=off`/per-call, or when the coprocess died this
   cycle). `_mbx_highlight_refresh` dispatches on `_MBX_ENGINE_READY`, exactly
   mirroring ghost's existing wire/CLI split.
5. Color on both paths is `_mbx_highlight_color_flag` (ADR 0015). Until
   ADR 0015, both paths passed `color=0` because Readline caret-renders
   `\001`/`\002` inside `READLINE_LINE` (`M-064`). Styled bytes now paint
   on a reserved row, so the interactive refresh sends a real color
   decision. `bind -x` stdout is often a pipe, so this flag is not
   `_mbx_color_capable`'s `-t 1` check. The `mbx highlight` CLI command
   itself gained `--color 0|1` as an explicit override (falling back to
   the pre-existing stdout-tty check when omitted) so a caller that
   already knows the true terminal capability can pass it correctly.
6. `_mbx_engine_write`/`_mbx_engine_exchange`'s existing SIGPIPE-isolated
   background write (`( trap '' PIPE; printf ... ) &`) is wrapped as
   `{ ( ... ) & } 2>/dev/null` (`M-063`). `set +m` alone does not suppress
   Bash's `[N] PID` job-start announcement for that backgrounded write when
   the caller is a `bind -x` keystroke callback — confirmed empirically: it
   does not happen when the same call runs from `PROMPT_COMMAND`, but does
   from self-insert — and the announcement is written to the shell's own
   stderr, not the backgrounded command's, so only redirecting the whole
   group's stderr suppresses it. This affects ghost's existing wire path too,
   not only highlighting's new one.

## Alternatives

- Route highlighting through a second coprocess or socket dedicated to
  low-latency features: unjustified extra process/lifecycle surface for one
  request type; the shared coprocess with generation-based stale-skip already
  handles the concurrency hazard.
- Reuse the `QUERY`/`RESULT` frame pair with a new `mode` instead of a new
  frame pair: rejected because `QUERY`/`RESULT`'s field shapes are
  history-specific (`mode`, search `text`, `limit`, candidate list) and do not
  fit highlighting's fields (`color`, cursor `point`, styled `line`) without
  overloading positions by convention; a dedicated frame pair keeps both
  narrow and self-documenting, matching this repository's interface
  segregation guidance.
- Fix `M-064` in the same change: rejected as out of scope. It requires
  determining whether any Readline-recognized technique makes styling
  genuinely invisible inside the edit buffer (as opposed to `PS1`), which is a
  rendering-strategy question, not a transport one, and deserves its own
  research slice and evidence rather than being bundled into a wire-protocol
  ADR.

## Consequences

Interactive highlighting no longer forks the helper binary once per
keystroke when a coprocess is attached; `crates/pty/tests/highlight.rs`
(`wire_highlight_forks_no_helper_process_per_keystroke`) asserts this
structurally via a counting shim, with a contrasting
`cli_fallback_highlight_does_fork_the_helper_per_keystroke` case proving the
shim itself is meaningful. `HLT-003`'s p99 latency leftover stays `deferred`
per `docs/latency-budget-deferral.md`, but the actual structural claim in the
roadmap's performance budget ("no external command on a cache-hit keystroke")
now has durable evidence for the coprocess path instead of being an unmet
percentile.

ADR 0015 resolved `M-064`: live color paints on the reserved preview row
and `READLINE_LINE` stays plain. Phase 6 and `HLT-003` (hostile/PTY gates;
p99 still `deferred`) closed on that path; see `docs/roadmap.md`.

## Validation

`docs/protocol-mbx2.md` HIGHLIGHT/STYLED section; unit tests in
`crates/cli/src/highlight_service.rs`; transport dispatch tests in
`crates/cli/src/transport.rs`
(`mbx2_highlight_frames_dispatch_to_the_highlight_handler_independently_of_history`,
`mbx2_without_highlight_handler_fails_closed_even_with_history_present`);
`crates/pty/tests/highlight.rs` full suite plus the two counting-shim cases;
`bash tests/bash/modules.bash`; `bash tests/run.bash`.
