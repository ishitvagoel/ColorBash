# Phase 3A vertical-slice contract (history sidecar, UI-free)

Status: draft for approval. `HIST-002` (PTY admission evidence) is complete;
when `HIST-001` (ADR 0005) is accepted, this document becomes the `HIST-003`
deliverable awaiting the approval decision.

## Purpose

The smallest coherent history slice that proves the capture, storage, and
deterministic-search contract end to end, with no editor UI. Everything here is
testable headlessly and independently of `G3`.

## Scope

In scope:

- opt-in Bash observation at the prompt boundary (the recorder port from
  `HIST-005`), emitting folded, admitted commands per ADR 0005 section 3;
- bounded protocol ingestion (MBX2 framing per ADR 0005 section 9) from the
  recorder to the helper;
- the narrow recorder/search/policy and reader/writer ports (`HIST-005`);
- SQLite storage: schema v1, migrations, `0700`/`0600` permissions, retention,
  one bounded writer queue and connection per helper session, with SQLite WAL
  serializing cross-session mutations (`HIST-006`, `HIST-012`, `HIST-013`);
- whole-record exclusions, disable/path/clear/delete controls (`HIST-011`);
- deterministic queries only: recent, exact-prefix, and cwd (`HIST-008`);
- idempotency via `(session_id, event_sequence)` and the ambiguity/drop rules.

Out of scope (later phases or explicit gates):

- fuzzy ranking (`HIST-009`), repository context (`HIST-010`);
- any editor UI, ghost text, or popup (needs `G2`/`G3`);
- modifying or reading `.bash_history` beyond the G2 invariance comparison;
- duration timing beyond the existing opt-in mechanism.

## Recording contract

- One record per Bash-admitted entry at the prompt boundary.
- Command text is the folded form Bash stores; NUL/invalid-UTF-8/empty/
  oversized entries are rejected without truncation (ADR 0005 section 3).
- Ambiguous attribution drops the record and increments a command-text-free
  diagnostic counter.
- `(session_id, event_sequence)` uniqueness is enforced by the store.

## Storage contract

- SQLite in WAL mode, `user_version` migrations, bounded `busy_timeout`.
- Writer owns all mutations; the prompt path enqueues only and never waits on
  locks; queue-full and storage errors drop enhancement data (ADR 0005
  sections 5–7; budgets in `docs/benchmarks/history-budgets.md`).
- Retention prunes in the writer with a configurable bounded default.

## Query contract (deterministic)

- recent: newest first, bounded result count;
- exact-prefix: `command_text LIKE 'prefix%'` with NOCASE prefix index, bounded
  result count;
- cwd: exact `start_cwd` match, newest first, bounded result count.
- Every query returns bounded rows (hard cap), is parameterized, and treats
  command text as inert data.

## Control contract

- disable, path inspection, clear, and delete per ADR 0005 section 5;
- no command text in any diagnostic, trace, or error (ADR 0005 section 8).

## Acceptance evidence

The slice is complete when:

1. `HIST-005`–`HIST-008` and `HIST-011`–`HIST-013` tests pass with the
   `HIST-004` corpus at the recorded budgets;
2. the PTY admission suite proves the recorder observes only admitted entries
   for every `HIST-002` case;
3. hostile SQL and terminal-control command text remains inert in storage and
   queries;
4. a same-command comparison shows no additional `.bash_history` changes
   (G2-level invariance);
5. no feature path writes `PS1`, executes command text, or blocks the prompt
   beyond the accepted budget.

## Non-goals for approval

Approval of this contract does not approve capture enablement; capture remains
gated on `G2`. It does not approve the MBX2 wire details; those are a separate
specification deliverable under `HIST-007`.
