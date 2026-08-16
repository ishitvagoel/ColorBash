# Phase 3A vertical-slice contract (history sidecar, UI-free)

Status: approved (2026-08-15). `HIST-002` (PTY admission evidence) is complete;
this document is the accepted `HIST-003` Phase 3A vertical-slice contract.

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

- One record per Bash-admitted entry at the prompt boundary after command
  completion; the first prompt is skipped so a seeded `HISTFILE` is not ingested.
- Command text is the folded form Bash stores; NUL/invalid-UTF-8/empty/
  oversized entries are rejected without truncation (ADR 0005 section 3).
- The diagnostic `history_number` is the `history 1` list number, not `HISTCMD`.
- Ambiguous attribution drops the record and increments a command-text-free
  diagnostic counter.
- `(session_id, event_sequence)` uniqueness is enforced by the store.

## Storage contract

- SQLite in WAL mode, `user_version` migrations, bounded `busy_timeout`.
- Writer owns all mutations; the prompt path enqueues only and never waits on
  locks; queue-full and storage errors drop enhancement data (ADR 0005
  sections 5–7; budgets in `docs/benchmarks/history-budgets.md`).
- Retention prunes in the writer with a configurable bounded default.

## Durability contract (HIST-012)

- **Queue model.** Each shell session has one bounded in-process queue between
  the recorder and its writer. Enqueue is the only prompt-side step: it pushes
  a record and returns; it never touches SQLite, never blocks on a lock, and
  never retries inside the prompt path. Queue capacity is bounded (configurable
  default), and the queue acknowledges by accepting the record, not by commit.
- **Acceptable loss.** Loss is confined to enhancement data, never to Bash
  behavior, `.bash_history`, or prompt correctness. Records are dropped, not
  retried, when: the queue is full; the writer is stopped; storage raises an
  unrecoverable error; or the observation deadline expires. Dropped records
  increment command-text-free diagnostic counters.
- **Writer drain.** The writer processes records in order, batches where
  practical, applies exclusions/retention, and commits with the bounded
  `busy_timeout`. It exits when the queue is closed and drained. A writer that
  fails a commit retries a bounded number of times, then drops the batch and
  continues; it must never terminate the helper.
- **Shell exit.** On interactive-shell exit, the recorder flushes the queue
  with the same bounded budget already used during the session. If the budget
  expires, remaining records are dropped silently (no blocking exit); the
  already-committed prefix is durable. No shell-exit path may wait on SQLite.
- **Crash.** Helper or shell crash may lose records still in the queue; the
  store remains consistent because each write is a single transactional insert
  keyed by `(session_id, event_sequence)`. Startup opens the store, verifies
  `user_version`, and recovers any WAL state without repairing data.
- **Retry.** Retries are permitted only at the queue drain / writer layer for
  transient SQLite errors (`SQLITE_BUSY`/`SQLITE_LOCKED` within `busy_timeout`
  or a bounded number of attempts). Idempotency keys make any replayed insert
  a no-op, so retries can never duplicate rows.
- **Storage failure.** If the store is unavailable or corrupt, the writer
  stops accepting new work, the queue drains by dropping, and the recorder
  degrades to no capture for the session; the prompt and shell are unaffected.
  A later session retries opening the store.

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

Approval of this contract did not approve default capture enablement; capture
remains off unless `MBX_HISTORY=1`. Default-on enablement remains a separate
product decision after `G2`.
MBX2 RECORD framing is specified in `docs/protocol-mbx2.md` and implemented for
this slice. Later MBX2 kinds (generation IDs, cancellation, search-over-the-wire)
remain out of scope.
