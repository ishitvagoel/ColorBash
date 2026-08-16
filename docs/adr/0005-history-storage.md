# ADR 0005: History sidecar — privacy, capture, data, and protocol contract

Status: Accepted (2026-08-15, G1 decision)

This ADR expands the earlier one-page sketch of the same record. The expanded
contract is the deliverable of roadmap item `HIST-001` and the acceptance test
for gate `G1`. Acceptance did not authorize default-on capture. The Phase 3A
path is implemented and remains off unless `MBX_HISTORY=1`; product enablement
still requires `G2` evidence.

## Context

Search and ranking need command text plus cwd, time, status, duration, host,
and session metadata. `.bash_history` is the compatibility source and encodes
user privacy choices through `HISTCONTROL`, `HISTIGNORE`, and history settings.
The product must enhance search without replacing Bash history, without logging
command text anywhere except a local, user-owned store, and without ever
blocking the interactive prompt.

PTY evidence in `docs/research/bash-history-admission.md` (`HIST-002`)
establishes how Bash actually admits entries: folding, filtering, renumbering,
and exit-flush behavior. This ADR binds the sidecar to that observed authority.

## Decision

### 1. Scope and authority

- The sidecar is optional and opt-in. Disabled by default; `.bash_history` is
  never written, truncated, or rewritten by MBX.
- Bash's resulting history list is the admission authority. The sidecar records
  command text only when Bash has admitted an entry at the prompt boundary after
  command completion, and it records the same folded, filtered text Bash stores.
- `$BASH_COMMAND`, `HISTCMD`, readline input, and `PROMPT_COMMAND` arguments are
  never used as the record source or as stable identifiers.
- `(session_id, event_sequence)` is the unique idempotency key, where
  `event_sequence` is a monotonic counter owned by the recorder (not `HISTCMD`
  and not the Bash list number). The diagnostic `history_number` is the list
  number printed by `history 1` for the newest admitted entry. `HISTCMD` is not
  stored: it is not a stable identifier and can be unset or still change while
  history is off.

### 2. Threat model and disclosure

- **Disclosure:** command text is sensitive. The store is plaintext SQLite
  local to the user, with filesystem permissions as the primary boundary. There
  is no encryption at rest and none is claimed.
- **Threats considered:** other local users reading the database (blocked by
  permissions), repository data or command text reaching terminal control
  (blocked by sanitization and parameterization), SQL injection from hostile
  command text (blocked by parameterized statements only), exfiltration
  (blocked by no network paths and no telemetry), and accidental leakage through
  logs (blocked by the no-command-text logging rule).
- **Out of scope:** protecting the store from the account owner, kernel-level
  attacks, or a compromised shell process.

### 3. Capture semantics (from `HIST-002` evidence)

- Commands Bash drops are dropped by the sidecar: leading-space under
  `ignorespace`, consecutive duplicates under `ignoredups`, `ignoreboth`
  combinations, `HISTIGNORE` matches, and all commands typed while history is
  disabled. The sidecar never fabricates an entry.
- Multiline commands are recorded in the folded single-entry form Bash stores
  (for example `if true; then echo x; fi`), never as separate lines.
- `history -s` injections appear in the sidecar because they appear in Bash's
  list; they are never executed by MBX.
- **Ambiguity rule:** if the recorder cannot match a completed command to an
  admitted history entry (for example renumbering races or concurrent
  mutation), it drops the record. A diagnostic counter increments; diagnostics
  never contain command text.
- **Drop rule:** commands exceeding the accepted maximum, containing NUL or
  invalid UTF-8, or empty are rejected without truncation and counted.

### 4. Recorded fields

| Field | Type | Notes |
| --- | --- | --- |
| `session_id` | UUIDv4 string | generated once per shell session |
| `event_sequence` | integer | monotonic recorder counter |
| `history_number` | integer | diagnostic list number from `history 1`, not an identifier and not `HISTCMD` |
| `command_text` | text | folded Bash-normalized command text |
| `start_cwd` | text | starting working directory |
| `completed_at` | text | completion timestamp (UTC ISO-8601) |
| `status` | integer | exit status of the completed command |
| `duration_ms` | integer or NULL | NULL when timing is disabled or unknown |
| `host` | text | hostname |
| `user` | text | username |

`status`/`duration_ms` attach at the prompt boundary after completion; an entry
whose status cannot be attributed drops per the ambiguity rule.

### 5. Storage, permissions, and lifecycle

- Path: `$XDG_DATA_HOME/mbx/history.sqlite3`, falling back to
  `$HOME/.local/share/mbx/history.sqlite3`.
- The `mbx` directory is created with mode `0700`; the database, WAL, and SHM
  files are created with mode `0600`; umask-visible permissions are verified by
  tests. Existing files are never made more permissive.
- Controls (implemented in `HIST-011`):
  - disable: setting or env var disables capture entirely; no store is created;
  - path inspection: report the store path without reading its contents;
  - clear: delete all rows, keeping the store;
  - delete: remove the database, WAL, and SHM files.
- Retention: configurable row/time cap with a default bounded value; pruning
  runs in the writer, never in the prompt path.

### 6. Schema, versioning, and migrations

- SQLite schema v1 with `PRAGMA user_version = 1`; migrations are forward-only
  and applied by the writer before use.
- Indexes: `(completed_at DESC)`, `(command_text COLLATE NOCASE)` prefix
  support, `(start_cwd)`, and the unique `(session_id, event_sequence)`.
- The schema stores values only through parameterized statements; command text
  is inert data, never SQL or terminal control.

### 6a. SQLite linkage decision (`HIST-013`)

- Linkage: `rusqlite` 0.32 with the `bundled` SQLite feature. Bundled linkage
  compiles SQLite into the binary, removing any dependency on a system SQLite
  dev package; this is the portability requirement for Linux, WSL, and macOS
  support.
- Measured cost on the development WSL2/Linux environment: release binary grew
  from 604,664 bytes to 2,626,336 bytes (+1.97 MiB) once the storage code is
  linked; a cold first build adds roughly 51 s for `libsqlite3-sys`; incremental
  rebuilds are unaffected. First-use latency and memory are bounded by the
  queue/writer design, not by the linkage.
- Packaging consequence: the bundled binary ships SQLite; no runtime library is
  required. This amends ADR 0002's standard-library-only stance for the
  measured history feature need.

### 7. Concurrency, durability, and idempotency

- Each Bash session has one bounded writer queue and writer connection. SQLite
  WAL plus its bounded `busy_timeout` serializes mutations across sessions; no
  shared history daemon is assumed for the MVP.
- The prompt path enqueues (bounded queue, acknowledgement p95 < 2 ms, p99 <
  5 ms) and never waits on database locks. The writer commits inserts in
  batches of 32 (`BEGIN IMMEDIATE` / `COMMIT`) before prune. Full queues and
  storage errors drop enhancement data according to the accepted durability
  contract in `HIST-012`. Retention limits are captured when the store is
  opened, not re-read on every prune.
- Retries use the `(session_id, event_sequence)` key so duplicates are
  impossible; concurrent shells never share an idempotency key.
- Retention, corruption, and lock-contention behavior are exercised by tests
  before `G2`.

### 8. Exclusions and secret policy

- Whole-record exclusions: env-var patterns (independent of `HISTIGNORE`, which
  remains Bash's own filter) remove a record before it is stored.
- Best-effort secret policy: the record is plaintext; MBX attempts no secret
  redaction inside stored command text. Documentation discloses this. Exclusion
  patterns are the supported mechanism for secret-bearing commands.
- No-command-text logging: diagnostics, traces, and error messages report kinds,
  counts, and paths only. Command text never enters telemetry, logs, or remote
  services.

### 9. Protocol decision: MBX2, not MBX1 extension

- History capture/search is **not** added to MBX1. MBX1 remains the bounded,
  prompt-oriented request/response protocol with additive flag bits.
- The interactive features need typed results, generation IDs, cancellation,
  and stale-response rejection. These are an incompatible framing/trust change
  and therefore become **MBX2**, per the boundary already stated in
  `docs/protocol.md`.
- MBX2 RECORD framing is specified in `docs/protocol-mbx2.md` and implemented
  for Phase 3A ingestion. The existing coprocess/socket transports and their
  bounds remain the framing baseline. Later MBX2 kinds are a later revision.

### 10. Sequencing

- No capture code ships before `G1` acceptance and the `HIST-003` slice
  contract.
- Writer/storage (`HIST-006`), exclusions/controls (`HIST-011`), Bash
  observation (`HIST-007`), and deterministic queries (`HIST-008`) each land
  behind the port boundaries defined in `HIST-005` and `HIST-012`/`HIST-013`.

## Alternatives

- Extending the flat history file cannot represent or query metadata safely.
- Replacing Bash history breaks tooling, shell behavior, and the `HISTCONTROL`
  contract.
- Remote or encrypted storage is outside MVP privacy and reliability bounds.
- Extending MBX1 ad hoc would silently change an accepted wire contract.

## Consequences

- Enhanced search can be disabled or deleted independently of Bash.
- Two stores require controlled deduplication: Bash history remains the
  compatibility source; the sidecar is the enhancement index.
- Users must be told that stored command text is plaintext local data.

## Risks

- Commands may contain secrets (mitigated by exclusions and disclosure).
- Concurrent shells can race (mitigated by idempotency keys, per-session writer
  queues, and SQLite's cross-session locking).
- Crash timing can mismatch status and command (mitigated by the ambiguity
  rule and prompt-boundary attribution).
- Database growth and migrations need bounds (retention and forward-only
  migrations).

## Validation plan

- `G1` (this ADR): threat model, capture semantics, schema, permissions,
  controls, and the MBX2 decision accepted with review.
- `G2` evidence: PTY admission suite (`HIST-002`, done); same-command
  `.bash_history` invariance comparison (`crates/pty/tests/history_invariance.rs`,
  done); hostile SQL/control inertness (done); 100k-row search p95 for recent,
  selective prefix, and cwd (`docs/benchmarks/2026-08-16-history-queries.md`);
  prompt-boundary write acknowledgement (`docs/benchmarks/2026-08-16-history-write-ack.md`;
  correctness recorded; percentile budget still open on development WSL);
  concurrent-writer contention; WAL crash/corrupt recovery (K-1–K-4 in
  `crates/cli/src/storage.rs`); WAL/SHM `0600` never-more-permissive (P-1–P-4
  in `crates/cli/src/storage.rs`); many-match prefix latency; foreign-user
  open; and command-text-free diagnostics.
- Every claim in this ADR maps to a test in `HIST-005`–`HIST-008`,
  `HIST-011`–`HIST-013` before `G2` passes.
