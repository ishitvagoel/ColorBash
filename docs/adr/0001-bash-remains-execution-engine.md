# ADR 0001: Bash remains the execution engine

Status: Accepted

## Context

MBX aims to modernize interactive editing and presentation without creating a new
shell language. Existing Bash configuration, expansion, jobs, traps, completion,
and scripts must retain their exact authority.

## Decision

Bash alone parses and executes command lines. The native helper receives bounded
context and returns presentation or candidate data. No protocol operation may ask
the helper to execute the current buffer. Future selections insert ordinary Bash
text and require the user's normal execution action.

## Alternatives

- A Rust shell/parser was rejected because Bash edge cases and configuration would
  diverge.
- A proprietary command graph was rejected because it hides shell operations.
- Executing Bash as a child of a custom editor remains research-only and cannot be
  adopted without parity evidence.

## Consequences

Compatibility has a clear authority and graceful helper failure is possible. Some
advanced UX is harder because Readline and Bash expose limited integration points.

## Risks

Presentation hooks can still accidentally alter `$?`, prompt callbacks, traps, or
terminal state. Untrusted text in PS1 can trigger expansion.

## Validation plan

Run the Bash semantic corpus before and after integration; add PTY tests for jobs,
signals, login/nested shells, and fullscreen applications. Reject any feature that
requires silent reinterpretation.

