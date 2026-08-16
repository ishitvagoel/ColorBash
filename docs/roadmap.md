# MBX delivery roadmap

> Canonical status: this is the single living source for delivery order, current
> status, dependencies, gates, and immediate next work. The originating product
> brief remains `CODEX_MODERN_BASH_ARCHITECTURE.md`; its checkboxes describe the
> intended program and are not a status tracker.

- Last reviewed: 2026-08-16 UTC
- Current milestone: Phase 3A sidecar implemented; `G0` and `G2` complete; `G1` accepted
- Active workstream: `G3` complete (`COMP-004` unblocked for planning; ghost/highlighting blocked on unproven continuous decoration)
- Next decision gate: continuous-decoration leftover (blocks ghost / highlighting)
- Editor-facing work is blocked by: unproven after-every-key decoration, as identified per phase
- Timing policy: unmet percentile targets are `deferred` and do not block
  product development (`docs/latency-budget-deferral.md`)

## How to maintain this roadmap

Use stable IDs when referring to work. Update this file in the same change that
alters scope, order, dependencies, status, or completion evidence.

Status values have precise meanings:

- `not-started`: approved scope, but work has not begun.
- `discovery`: research or an experiment must resolve design uncertainty.
- `ready`: dependencies and decisions are satisfied; implementation may start.
- `in-progress`: implementation or documentation is actively being changed.
- `validation`: an implementation exists, but its exit evidence is incomplete.
- `complete`: every exit criterion has durable linked evidence.
- `blocked`: a named dependency prevents meaningful progress.
- `deferred`: intentionally outside the current milestone.
- `superseded`: replaced by a referenced decision or deliverable.

Do not use percentages or an ambiguous `partial` status. Code existing is not
enough for `complete`. Record UTC dates and a commit or PR when one exists. Move
removed work to `deferred` or `superseded` instead of silently deleting it.

## Source-of-truth boundaries

| Source | Authority |
| --- | --- |
| `CODEX_MODERN_BASH_ARCHITECTURE.md` | Product constraints, long-term intent, MVP scope, and stable phase numbering |
| `docs/roadmap.md` | Current plan, order, status, gates, and next work |
| `docs/architecture.md` | What the implementation currently is and where its boundaries are |
| `docs/adr/` | Why consequential decisions are proposed, accepted, or superseded |
| `docs/bash-compatibility.md` | Bash semantics and compatibility contract |
| `docs/protocol.md` | MBX1 wire behavior, encoding, bounds, and protocol security contract |
| `docs/protocol-mbx2.md` | MBX2 history-record wire behavior |
| Code plus reproducible tests | Observed runtime behavior; they do not redefine product intent |
| Tests and benchmark records | Evidence required to pass gates and mark work complete |
| `MISTAKES.md` | Confirmed lessons and prevention rules from prior work |

Product constraints and accepted ADRs govern intent. Code plus reproducible tests
establish observed behavior. When implementation and documentation disagree,
make the discrepancy explicit and correct stale architecture/protocol/status
documents when authorized. This roadmap cannot redefine implementation truth or
silently overwrite an accepted decision.

## Non-negotiable delivery invariants

1. Bash remains the parser, executor, expansion engine, and job controller.
2. Existing Bash commands and configuration retain their semantics.
3. Suggestions insert text and never execute automatically.
4. Helper failure always degrades to a usable Bash experience.
5. The hot path remains bounded and responsive; severe latency is correctness.
6. Untrusted data never controls terminal escapes, `PS1` expansion, SQL syntax,
   subprocess executable selection, or option structure. Values may be passed
   only through safe data/argument boundaries.
7. `.bash_history` remains untouched by the optional sidecar.
8. Command text never enters telemetry, diagnostics, or remote services.
9. Readline keeps editing and redisplay authority until evidence and an ADR say
   otherwise.
10. Major protocol, persistence, privacy, dependency, or ownership changes require
    an ADR or an explicit update to an existing ADR.

## Current baseline

The repository is a foundation prototype plus a UI-free history sidecar, not the
MVP. A green GitHub Actions CI run is linked for `FND-001`
(`docs/fnd-001-ci-plan.md`). `G0` and `G2` are complete. Remaining prompt
percentile matrix and write-ack p95/p99 are `deferred`
(`docs/latency-budget-deferral.md`). `HRD-001` macOS remains release-matrix
work, not a product-development gate.

Implemented foundation:

- a small interactive-only Bash loader with idempotent sourcing and
  status-preserving prompt hooks;
- separate Bash protocol, configuration, engine, coordinator, fallback, hook,
  and history-observation modules with one `PS1` writer;
- a thin Rust composition root and separate CLI, environment, application,
  service, prompt, provider, history, storage, transport, and telemetry modules;
- narrow `RequestHandler`, `PromptRendering`, `PromptSegmentProvider`,
  `RepositoryStatusProvider`, and history policy/recorder/search/control ports;
- the MBX1 prompt protocol with a 64-KiB acceptance rule, correlation IDs, typed
  prompt flags, lossless additive-flag forwarding, bounded cross-language field
  encoding, and terminator-independent acquisition limits;
- MBX2 RECORD ingestion for opt-in history capture, sharing MBX1 framing bounds;
- an adaptive prompt with path, Git status, nonzero status, optional duration,
  SSH, production context, semantic roles, and icon/color fallbacks;
- coprocess, per-call, and process-free Bash-only degradation paths sharing one
  render deadline;
- a fixed-spec Git provider with capped acquisition, a 50-ms refresh deadline,
  typed failure diagnostics, and a bounded one-second warm cache;
- an opt-in SQLite history sidecar with schema v2 (forward-only from v1), WAL, `0700`/`0600`
  permissions, retention, exclusions, path/count/clear/delete controls, and
  deterministic recent/prefix/cwd queries; capture remains off unless
  `MBX_HISTORY=1`;
- Rust unit tests plus Bash module, protocol-integration, compatibility smoke,
  and genuine PTY driver/foundation/history suites; and
- architecture, UX, compatibility, protocol, research, benchmark, and ADR
  documentation.

The canonical local check is `bash tests/run.bash`. It passed in the final
hardening working tree on 2026-08-15, with focused evidence recorded in
`docs/solid-hardening-checklist.md` and release measurements in
`docs/benchmarks/2026-08-15-solid-hardening.md`. GitHub Actions workflow `CI`
recorded a green run on `origin/main` at commit
`8c8dad24d46d75d5eb311bacc06a0e2e25b5c5a9`
(https://github.com/ishitvagoel/ColorBash/actions/runs/31937499009), completing
`FND-001` and `BST-005`. `G0` is complete; remaining percentile leftovers are
`deferred`.

Not implemented:

- fuzzy history ranking or repository-context history fields;
- enhanced Ctrl+R, ghost suggestions, completion UI, or live highlighting;
- arbitrary key-injection coverage (Tab, arrows, Ctrl+R), the release platform
  matrix, or remaining `G0` platform-matrix evidence;
- prompt-boundary write-ack percentile budget (correctness recorded; p95 miss
  deferred from `G2` — `docs/history-g2-write-ack-deferral.md`); or
- asynchronous feature IPC or the broader completion/history provider model.

Known foundation debt:

- the Git runner guarantees capped output; its normal timeout path attempts
  direct-child kill/reap and types cleanup failures, but does not guarantee
  portable process-tree termination. An unexpected descendant may outlive the
  provider without extending prompt return, and kernel stalls in process-
  management calls are not independently cancellable;
- a nonzero fixed Git worktree preflight maps to absence, so rare fatal discovery
  failures remain indistinguishable from a non-repository without acquiring
  stderr; absolute `PATH` entries are trusted caller configuration;
- the one-budget Bash deadline is enforced cooperatively between bounded builtin
  operations, so one in-progress read chunk or native pattern match may overshoot
  by a small amount;
- the controlled warm-Git benchmark passes provisional targets, but representative
  dirty/large repositories, cold refresh, fallback, PTY, and platform percentile
  evidence are `deferred` `PRM-004` leftovers (`docs/latency-budget-deferral.md`);
- the display-width helper compacts paths at 52 display columns and color
  capability (16/256/truecolor) is recorded; non-DSR wrap-column PTY usability
  is recorded (`docs/prm-002-wrap-column-plan.md`); PTY round-trip evidence
  exists for wide/combining glyphs and resize;
- the PTY driver in `crates/pty/src/sys.rs` is Linux/WSL-accurate; Darwin
  `O_CLOEXEC`, `poll`'s `nfds_t`, and `ptsname_r` are cfg-split with cited
  header values (`docs/hrd-001-darwin-pty-constants-plan.md`); the full
  `HRD-001` macOS PTY matrix still requires a macOS host (`O_NOCTTY` and
  `Termios`/`TIOC*` were already cfg-split);
- duration timing is opt-in because composing arbitrary DEBUG traps is unsafe;
- tracing is intentionally minimal;
- the experimental socket server is sequential, and abrupt termination can leave
  a socket path; and
- direct `mbx prompt` defaults disable color when stdout is not a terminal;
  explicit `--flags` still forwards caller capability (`docs/prm-002-redirected-output-plan.md`);
  the display-width helper compacts long paths at 52 columns; color capability
  (16/256/truecolor) is recorded; non-DSR wrap-column PTY usability is recorded
  (`docs/prm-002-wrap-column-plan.md`).

## Dependency and gate map

```text
Current SOLID foundation
        ├── completed prompt/provider hardening ──── G0 foundation stability
        ├── PTY + Readline insertion experiment ──── G3 editor integration
        ├── completion-adapter experiment ────────── G4 completion parity
        └── G1 history privacy ── Phase 3A ───────── G2 history readiness
                                                        │
                         G2 + G3 ── enhanced Ctrl+R ─────┤
                         G2 + G3 ── ghost suggestions ───┤
                         G3 + G4 ── completion popup ────┤
                              G3 ── highlighting last ───┤
                                                        │
                                      all MVP exits ── G5 release hardening
```

Phase numbers preserve the original brief. Actual execution order follows the
dependencies above; Phase 8 enhanced Ctrl+R is intentionally prototyped before
continuous Phase 4 ghost rendering because an explicit insertion action is a
lower-risk validation of search, restoration, and exact text insertion.

## Gates

### G0 — Foundation stability

Status: `complete` (2026-08-16)

Functional foundation evidence is recorded. Remaining prompt percentile matrix
work is `deferred` and does not block product development
(`docs/latency-budget-deferral.md`). `HRD-001` macOS stays Phase 9 / `G5`
release-matrix work.

Passed when:

- the current modular refactor is reviewed and landed as a distinct baseline;
- the canonical suite is green on a clean tree and CI evidence is linked;
- MBX1 request/response acquisition, correlation, and `MAX-1`/`MAX`/`MAX+1`
  behavior are bounded and consistent for EOF, LF, and CRLF in Rust and Bash;
- native and fallback prompt adapters satisfy the same explicit context, flag,
  terminal-safety, and bounded-failure contract;
- a genuine PTY harness covers the foundation prompt lifecycle, helper failure,
  Ctrl+C, Ctrl+Z, and resize at minimum;
- complete prompt rendering has a controlled warm-Git p50/p95/p99 record;
  remaining fallback/dirty/large/cold/PTY/platform percentiles are `deferred`;
- Git acquisition has a deadline, streaming output bound, and warm cache; and
- remaining accepted prototype debt is explicitly deferred with an owner/gate.

### G1 — History privacy and data contract

Status: `complete` (2026-08-15)

ADR 0005 is accepted with a threat model and plaintext-local storage disclosure;
authoritative Bash capture semantics and ambiguity/drop behavior; schema,
versioning, migrations, retention, concurrency, and idempotency; whole-record
exclusions, best-effort secret policy, and no-command-text logging; storage
path, `0700` directory and user-only database/WAL/SHM permissions; disable,
path inspection, clear, and deletion behavior; and the explicit MBX2 protocol
decision. History capture is implemented and remains off unless `MBX_HISTORY=1`.
Default-on product enablement remains a separate decision; capture stays off
unless `MBX_HISTORY=1`.

### G2 — History sidecar and search readiness

Status: `complete` (2026-08-16)

The UI-free Phase 3A slice and required `G2` evidence are recorded. The
prompt-boundary write-ack **percentile** leftover is `deferred` (not a budget
pass): W-1–W-4 correctness is recorded; WSL and cloud p95 miss the provisional
2 ms / 5 ms budget (`docs/history-g2-write-ack-deferral.md`). Revisit that
leftover later; do not weaken the documented numbers.

Passed when:

- a controlled same-command comparison shows that enabling, disabling, clearing,
  and deleting the sidecar causes no additional `.bash_history` changes beyond
  Bash's normal behavior;
- Bash omissions, history-off cases, duplicates, multiline entries, renumbering,
  and ambiguous capture cases have PTY evidence;
- starting cwd, completion timestamp, status, nullable duration, session ID, and
  event sequence are correct;
- concurrent shells, retries, migrations, retention, corruption, lock contention,
  and permission behavior are tested;
- hostile SQL and terminal control data remains inert;
- privacy exclusions, disable/path/clear/delete controls, and command-text-free
  diagnostics are implemented and tested;
- 100k+ row search benchmarks meet the accepted query budgets; write-ack
  percentile pass is `deferred` (`docs/history-g2-write-ack-deferral.md`); and
- storage failure or queue saturation stays inside the accepted prompt-side
  budget and never breaks the shell.

`G2` is the Phase 3A gate. It permits history-driven editor experiments but does
not mark full Phase 3 complete; full completion also requires the later fuzzy
ranking and repository-context deliverables identified in that phase.
It requires `HIST-002` through `HIST-008` plus `HIST-011`, `HIST-012`, and
`HIST-013`; `HIST-009` and `HIST-010` remain full-phase exit work.

### G3 — Editor integration feasibility

Status: `complete` (2026-08-16)

`EDT-001` produced this gate. A configurable, non-destructive `bind -x`
prototype:

- reads and updates `READLINE_LINE`/`READLINE_POINT` without executing text;
- preserves exact bytes, cursor position, suffixes, quoting, and multiline input;
- does not overwrite existing user bindings without explicit configuration;
- works in emacs and vi modes with bracketed paste, resize, Ctrl+C, and Ctrl+Z;
- restores terminal state and prompt output after cancellation/failure; and
- demonstrates insert-time Readline redraw without printable-key rebinds
  (B-5). Continuous after-every-key decoration stays unproven and still
  blocks ghost and highlighting (`docs/g3-gate-close-plan.md`; ADR 0003).

### G4 — Completion parity

Status: `complete` (2026-08-16)

The non-popup `COMP-001`/`COMP-002` adapter experiment produced this gate. It may
run alongside `G3`, using the shared PTY harness, but a custom completion menu is
blocked until `COMP-004`. Passed when file completion and at least one existing
`-F` completion function preserve stock Bash behavior for:

- `COMP_*` inputs, `COMPREPLY`, and `compopt` effects;
- exact candidate bytes, quoting, escaping, whitespace, and suffix insertion;
- aliases, redirections, Unicode, incomplete quotes, `--`, and nested commands;
- unsupported, slow, or stateful completion functions through safe fallthrough;
  and
- the accepted latency budget over the original completion function.

Functional parity evidence is recorded in `docs/g4-decision-plan.md` and
`docs/g4-gate-close-plan.md`. The provisional 5 ms adapter overhead leftover
stays `deferred` per `docs/latency-budget-deferral.md` and does not block gate
close (same precedent as `G2` write-ack deferral).

### G5 — MVP release

Status: `blocked` by unproven continuous decoration and the MVP phase exits

Requires `GHST-004`, `COMP-005`, `HLT-003`, `GIT-004`, `SRCH-003`, and all
`HRD-*` release deliverables, plus real-PTY evidence across the supported Bash/OS
matrix, hostile-input tests, signals and resize, tmux/SSH/fullscreen applications,
helper crashes, terminal restoration, installation/removal, and all accepted
latency budgets.

## Phase summary

| Phase | Name | Status | Principal unfinished condition |
| ---: | --- | --- | --- |
| 0 | Research / architecture | `complete` | `G0` complete; `HRD-001` macOS remains release-matrix work |
| 1 | Bootstrap | `complete` | CI linked; broader lifecycle tracing deferred |
| 2 | Prompt | `complete` | capability/width/wrap recorded; `PRM-004` percentiles `deferred` |
| 3 | History | `complete` | Phase 3A slice implemented; `G2` complete; write-ack percentiles `deferred` |
| 4 | Ghost suggestions | `blocked` | unproven continuous decoration |
| 5 | Completion | `validation` | `G4` / `COMP-003` complete; `COMP-004` unblocked for planning |
| 6 | Syntax highlighting | `blocked` | unproven continuous decoration; intentionally after search/ghost/completion evidence |
| 7 | Git/provider expansion | `discovery` | bounded prompt subset exists; richer capabilities await a consumer |
| 8 | Enhanced Ctrl+R | `blocked` | unproven continuous decoration leftover |
| 9 | Release hardening | `not-started` | feature gates and full compatibility matrix |

## Phase details

### Cross-cutting foundation work

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `FND-001` | Review and land the SOLID refactor as a clean baseline | `complete` | green GitHub Actions `CI` run on `origin/main` at `8c8dad2` (https://github.com/ishitvagoel/ColorBash/actions/runs/31937499009); `docs/fnd-001-ci-plan.md` |
| `FND-002` | Make transport own response correlation/framing postconditions and test `RequestHandler` substitutes directly | `complete` | `crates/cli/src/service.rs`, `transport.rs`, and direct substitute/oversize/correlation tests |
| `FND-003` | Complete port-contract tests for full prompt mapping, ping isolation, provider error/disable behavior, and crate-internal seam construction | `complete` | service, prompt-provider, disabled-provider, and sibling seam tests in `crates/cli/src/` |
| `PTY-001` | Genuine PTY driver for input, signal, resize, and terminal-state probes | `complete` | `crates/pty` driver tests plus foundation prompt/helper/Ctrl+C/Ctrl+Z/resize/`stty -g` coverage |
| `EDT-001` | Non-destructive `bind -x` insertion/redisplay feasibility prototype | `complete` | E-1–E-4, M-1–M-4, B-1–B-4 in `crates/pty/tests/editor_bind_x.rs` and `bash/editor.bash`; B-5 redraw note; `docs/g3-gate-close-plan.md`; continuous decoration leftover unproven |

### Phase 0 — Research and architecture

Outcome: prove the hybrid Bash/Rust foundation without changing Bash execution.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `RSH-001` | Architecture, UX, compatibility, and initial ADR set | `complete` | `docs/architecture.md`, `docs/ux-spec.md`, `docs/bash-compatibility.md`, `docs/adr/` |
| `RSH-002` | Bash lifecycle, Readline, and completion investigation | `complete` | `docs/research/bash-readline-investigation.md` |
| `RSH-003` | Per-call, coprocess, and socket transport comparison | `complete` | ADR 0004 and `docs/benchmarks/2026-08-15-ipc.md` |
| `RSH-004` | Multiline/display-width/resize PTY validation | `complete` | `crates/pty/tests/multiline_width.rs` and `docs/research/multiline-width-pty.md` |
| `RSH-005` | Reassess editor architecture with experimental evidence | `superseded` | tracked explicitly by `G3` and `G4` |

Exit condition: `G0`. Editor go/no-go evidence is a later pre-editor condition in
`G3` and `G4`; it does not make the Phase 0 exit circular.

### Phase 1 — Bootstrap

Outcome: safe installation/loading, explicit IPC, observable failure, and graceful
degradation.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `BST-001` | Rust workspace, Bash loader, and development setup | `complete` | `Cargo.toml`, `bash/init.bash`, `scripts/dev-setup.bash` |
| `BST-002` | Interactive guard, idempotence, status preservation, and fallback | `validation` | shell suites plus PTY lifecycle/failure tests; platform matrix remains `HRD-001` |
| `BST-003` | MBX1 coprocess and per-call adapters | `validation` | bounded protocol/module tests pass; PTY helper-crash coverage exists; platform matrix remains |
| `BST-004` | Debug/trace logging without command text | `validation` | minimal Rust trace exists; broader lifecycle diagnostics deferred |
| `BST-005` | CI and canonical verification suite | `complete` | `.github/workflows/ci.yml` runs `bash tests/run.bash`; green run https://github.com/ishitvagoel/ColorBash/actions/runs/31937499009 on `origin/main` at `8c8dad2` |
| `BST-006` | Enforce a terminator-independent 64-KiB boundary and cap Bash response acquisition before allocation | `complete` | Rust/Bash `MAX-1`/`MAX`/`MAX+1` EOF/LF/CRLF, NUL, and oversized-producer tests |
| `BST-007` | Prove socket collision refusal, `0600` mode, cleanup, and correlation behavior | `complete` | focused Unix tests cover collisions, mode, cleanup ordering, and mismatched IDs |

Exit condition: `G0`.

### Phase 2 — Prompt

Status: `complete` for `G0` prompt requirements (2026-08-16). `PRM-004`
remaining percentiles are `deferred`.

Outcome: an adaptive, semantic, safe prompt that remains fast under failure.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `PRM-001` | Path, Git, status, duration, SSH, production, icon, and theme segments | `validation` | Rust/Bash renderers and semantic tests |
| `PRM-002` | Capability, redirected-output, visible-width, and resize model | `complete` | redirected-output color policy recorded (`docs/prm-002-redirected-output-plan.md`; `crates/cli/src/environment.rs`); display-width path compaction helper recorded (`docs/prm-002-width-plan.md`; `crates/cli/src/prompt.rs`); color capability (16/256/truecolor) recorded (`docs/prm-002-color-capability-plan.md`; `crates/cli/src/environment.rs`, `crates/cli/src/prompt.rs`, `bash/config.bash`, `bash/fallback.bash`); non-DSR wrap-column PTY usability recorded (`docs/prm-002-wrap-column-plan.md`; `crates/pty/tests/multiline_width.rs`) |
| `PRM-003` | End-to-end deadline, capped Git acquisition, and warm cache | `complete` | one Bash deadline, capped 50-ms Git refresh, 128-entry/1-s cache, failure tests, and `docs/benchmarks/2026-08-15-solid-hardening.md` |
| `PRM-004` | Full prompt p50/p95/p99 benchmark matrix | `deferred` | controlled warm-Git case recorded; remaining matrix deferred (`docs/latency-budget-deferral.md`) |
| `PRM-005` | Real PTY prompt, wrap, signal, resize, and restoration tests | `complete` | lifecycle, helper failure, Ctrl+C/Z, resize, `stty -g`, multiline, narrow wrap, and wide-glyph coverage |
| `PRM-006` | Decide opt-in duration policy or explicit preexec adapter | `validation` | `docs/prm-006-duration-plan.md`; D-1–D-4 in `tests/bash/smoke.bash`; remain opt-in; do not compose `DEBUG` |
| `PRM-007` | Give native and fallback adapters one explicit input/safety contract and shared hostile-state corpus | `complete` | explicit four-field context, shared C0/DEL/expansion corpus, production precedence, and SSH-only test |
| `PRM-008` | Preserve raw additive prompt flags across coprocess, per-call, and fallback paths | `complete` | raw `--flags` CLI boundary plus coprocess/per-call/fallback unknown-bit tests |
| `PRM-009` | Reassess semantic composition versus typed PS1 encoding and validated theme styles | `discovery` | wait for `PRM-002` or a second renderer; avoid a speculative abstraction |

Exit condition: prompt requirements of `G0`.

### Phase 3 — History

Status: `complete` for the UI-free Phase 3A / `G2` slice (2026-08-16).
`HIST-009` and `HIST-010` remain full-phase exit work. Write-ack percentiles
are `deferred` (`docs/history-g2-write-ack-deferral.md`).

Outcome: an opt-in, local sidecar that records Bash-approved history metadata and
provides bounded search without modifying `.bash_history`, without synchronous
storage waits, and within the accepted prompt-side observation budget.

The candidate Phase 3A vertical slice is deliberately UI-free. ADR 0005 must
accept or revise these details before implementation:

- Bash's resulting history entry is the admission authority; `$BASH_COMMAND` is
  not a substitute and `HISTCMD` is not a stable identifier.
- Capture Bash-normalized command text, starting cwd, completion timestamp, exit
  status, nullable duration, session ID, event sequence, and the diagnostic
  `history 1` list number (not `HISTCMD`).
- Use unique `(session_id, event_sequence)` retry idempotency.
- Prefer `$XDG_DATA_HOME/mbx/history.sqlite3`, falling back to
  `$HOME/.local/share/mbx/history.sqlite3`.
- Use a bounded queue and per-session writer so the prompt does not
  synchronously wait on database locks. Full queues and storage errors drop
  enhancement data according to the accepted durability contract.
- Reject NUL, invalid UTF-8, empty, and oversized commands without truncation.
- Begin with deterministic recent, exact-prefix, and cwd queries. Add fuzzy
  ranking only over a bounded candidate set.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `HIST-001` | Expand and accept ADR 0005 threat/data/protocol contract | `complete` | ADR 0005 accepted; `G1` passed |
| `HIST-002` | PTY characterize Bash admission and multiline behavior | `complete` | `crates/pty/tests/history_admission.rs` and `docs/research/bash-history-admission.md` |
| `HIST-003` | Approve the Phase 3A vertical-slice contract | `complete` | `docs/history-phase3a-contract.md` approved |
| `HIST-004` | Define datasets, contention cases, and benchmark budgets | `complete` | `docs/benchmarks/history-budgets.md` |
| `HIST-005` | Add narrow recorder/search/policy and reader/writer ports | `complete` | `crates/cli/src/history.rs` ports plus policy and history-service substitutes |
| `HIST-013` | Decide SQLite linkage/dependency and supported-platform packaging | `complete` | bundled rusqlite; measured +1.97 MiB release binary and first-build cost in ADR 0005 section 6a |
| `HIST-012` | Define queue drain, shell-exit, crash, retry, and acceptable-loss semantics | `complete` | durability contract in `docs/history-phase3a-contract.md`; writer batches busy queues to 32 and idle-flushes partial batches; Shutdown drains |
| `HIST-006` | Implement SQLite schema, migrations, permissions, retention, and writer | `complete` | `crates/cli/src/storage.rs` schema v1, WAL, `0700`/`0600`, retention prune, batched writer |
| `HIST-011` | Implement exclusions, no-log policy, disable/path/clear/delete controls | `complete` | `crates/cli/src/policy.rs` plus `mbx history path|count|clear|delete` and env controls |
| `HIST-007` | Add opt-in Bash observation and bounded protocol ingestion | `complete` | `bash/history.bash`, MBX2 RECORD ingestion, PTY recording/invariance tests, seeded 100k corpus, hostile inertness, query p95, concurrent-writer contention, prompt-boundary write-ack correctness, WAL crash/corrupt recovery, WAL/SHM `0600` never-more-permissive, many-match prefix covering index, writer idle-flush for live readers, 100k-row v1→v2 migration (`crates/cli/src/corpus.rs`), and foreign-user open (`docs/history-g2-foreign-user-plan.md`; F-1–F-4 in `crates/cli/src/storage.rs`); write-ack percentile leftover `deferred` (`docs/history-g2-write-ack-deferral.md`) |
| `HIST-008` | Add recent, exact-prefix, cwd queries and deterministic ranking | `complete` | `mbx history search recent|prefix|cwd` with bounded limits and NOCASE prefix index |
| `HIST-009` | Add bounded fuzzy ranking | `blocked` | `HIST-008`, deterministic-query evidence, and 100k+ benchmark |
| `HIST-010` | Add repository context | `blocked` | history-scoped `GIT-003` root/branch provider subset |

Exit conditions: `G1` before capture can be enabled; `G2` before history-driven
editor UI; full Phase 3 completion additionally requires `HIST-009` and
`HIST-010`. Deferring either beyond the MVP requires an accepted scope/ADR
decision and corresponding reconciliation of the authoritative product brief;
the roadmap cannot make that scope change by itself.

### Phase 4 — Ghost suggestions

Status: `blocked` by unproven continuous decoration.

After the gates pass, implement asynchronous ranked-history lookup with generation
IDs, stale-result rejection, inline rendering, full/word acceptance, cycling, and
multiline/resize behavior. Acceptance must preserve exact bytes/cursor position,
never execute the suggestion, and perform no external command on a cache-hit
keystroke.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `GHST-001` | Async ranked query with generation IDs and cancellation | `blocked` | async IPC ADR decision |
| `GHST-002` | Inline ghost rendering with stale-result rejection | `blocked` | unproven continuous decoration, `GHST-001` |
| `GHST-003` | Full/word acceptance and suggestion cycling | `blocked` | `GHST-002` |
| `GHST-004` | Multiline, resize, exact-byte, no-execution, and latency evidence | `blocked` | `GHST-003`, `PTY-001` |

Exit condition: `GHST-004` meets the accepted editing and safety budgets.

### Phase 5 — Completion

Status: `validation`; popup is unblocked for planning (`COMP-004`).

First adapt stock Bash completion and prove exact insertion parity. Only then add
typed candidate metadata, bounded ranking, popup navigation, and Git candidates.
Unsupported completion specifications must fall through without mutating the
line. Do not move completion functions into a subprocess unless live-state and
`compopt` parity are demonstrated.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `COMP-001` | Build a non-popup stock-completion adapter harness | `validation` | `docs/comp-001-harness-plan.md`; H-1–H-4 in `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs`; `G4` complete |
| `COMP-002` | Prove file and one `-F` function's exact insertion parity | `validation` | `docs/comp-002-parity-plan.md`; P-1–P-4, F-1–F-4, L-1–L-4, N-1–N-2, S-1–S-4 landed; `docs/g4-gate-close-plan.md`; 5 ms leftover `deferred` |
| `COMP-003` | Add typed candidate metadata and bounded ranking | `complete` | `docs/comp-003-metadata-plan.md` K-1–K-4; `docs/comp-003-ranking-plan.md` R-1–R-4 in `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs` |
| `COMP-004` | Add popup navigation and terminal-safe rendering | `not-started` | `G3` / `COMP-003` complete; separate slice — do not bundle with gate close |
| `COMP-005` | Insert/fall through exactly and pass the parity/PTY matrix | `blocked` | `COMP-004`, Git candidates when enabled |

Exit condition: `G4` for the adapter slice; `COMP-005` for the MVP completion
feature.

### Phase 6 — Syntax highlighting

Status: `blocked` by unproven continuous decoration and intentionally last among editor experiments.

Define a tolerant token taxonomy only after Readline redraw feasibility is known.
The highlighter must accept incomplete Bash, never execute or expand input, bound
work by input size, classify dangerous text visually without blocking execution,
and strip back to the exact original bytes.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `HLT-001` | Define token taxonomy and tolerant incomplete-input lexer | `blocked` | unproven continuous decoration and ADR 0003 |
| `HLT-002` | Integrate terminal-safe styling without taking execution ownership | `blocked` | `HLT-001`, accepted redraw strategy |
| `HLT-003` | Pass exact-byte stripping, hostile-input, PTY, and latency gates | `blocked` | `HLT-002`, `PTY-001` |

Exit condition: `HLT-003`.

### Phase 7 — Git and provider expansion

Status: `discovery`; a synchronous prompt-status subset was pulled forward.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `GIT-001` | Typed prompt repository-status provider | `complete` | `crates/cli/src/provider.rs`, ADR 0007, and provider substitution/degradation tests |
| `GIT-002` | Deadline, capped acquisition, TTL cache, refresh, invalidation | `complete` | ADR 0007, provider outcome/process/cache tests, and `docs/benchmarks/2026-08-15-solid-hardening.md` |
| `GIT-003` | Repository root/branch context, then upstream/branches/remotes/tags | `blocked` | `HIST-010` is the first scoped consumer; no expansion is authorized yet |
| `GIT-004` | Structured completion metadata/ranking | `blocked` | Phase 5 candidate model (`COMP-003`) |
| `GIT-005` | General provider capabilities/SDK | `deferred` | post-MVP evidence; ADR 0007 update required |

Do not add Python, Node, Docker, arbitrary executable plugins, or a generic SDK
until the MVP's concrete provider consumers establish the required contracts.

Exit conditions: `GIT-002` for the safe prompt-provider slice; `GIT-004` for the
MVP Git/completion slice. `GIT-005` remains explicitly post-MVP.

### Phase 8 — Enhanced Ctrl+R

Status: `blocked` by unproven continuous decoration, but first in the editor-feature sequence once
that leftover is resolved.

Build a configurable explicit search action with age, cwd, and useful status
metadata; bounded filtering; safe cancellation; exact insertion without
execution; and terminal restoration. Repository/failed-command filters are added
only when their indexed fields are reliable.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `SRCH-001` | Configurable bounded history-search action and result view | `blocked` | unproven continuous decoration |
| `SRCH-002` | Cancel restoration and exact insertion without execution | `blocked` | `SRCH-001`, `PTY-001` |
| `SRCH-003` | Metadata filters, 100k-row latency, signal, and terminal-state evidence | `blocked` | `SRCH-002`; repository filters also need `HIST-010` |

Exit condition: `SRCH-003`.

### Phase 9 — Release hardening

Status: `not-started` as a release campaign. Foundation tests already provide a
starting corpus, but they do not satisfy `G5`.

The final matrix must include supported Bash 5.x releases; Linux, WSL, and macOS;
interactive/login/nested shells; emacs/vi modes; tmux and SSH; 16/256/true color;
Unicode and plain text; helper crashes; timeout/malformed IPC; Ctrl+C/Ctrl+Z;
resize and wrapping; background jobs; common prompt/preexec frameworks; and
representative fullscreen applications. Use pairwise coverage where exhaustive
combinations are impractical.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `HRD-001` | Supported Bash/OS/terminal pairwise PTY matrix | `not-started` | all MVP feature exits; Darwin PTY constant pre-work (D-1–D-3) recorded in `docs/hrd-001-darwin-pty-constants-plan.md`; macOS matrix run still required |
| `HRD-002` | Hostile input, protocol bounds, privacy, and no-execution audit | `not-started` | feature-complete candidate |
| `HRD-003` | Release-mode end-to-end latency and resource evidence | `not-started` | accepted workloads/budgets |
| `HRD-004` | Install, upgrade, disable, removal, crash, and recovery evidence | `not-started` | release packaging |

Exit condition: `G5` after every `HRD-*` item is complete.

## Immediate next work

`G0` and `G2` are complete. Capture stays disabled by default. Unmet
percentile leftovers are `deferred` and must not block product slices
(`docs/latency-budget-deferral.md`).

1. `G3` is `complete` (`docs/g3-gate-close-plan.md`). Continuous decoration
   stays unproven and still blocks ghost and highlighting. Do not start
   popup, ghost, or `COMP-004`. `EDT-001` is `complete`. Do not mark
   `COMP-001` or `COMP-002` complete.
2. `HRD-001` macOS PTY matrix remains Phase 9 / `G5` work. Do not spend a
   slice on FND-001 SHA refresh or percentile benches unless a functional
   prompt-path defect is proven.

## Provisional performance and safety budgets

These are planning targets for later review (`docs/latency-budget-deferral.md`).
They must not block product development. Do not weaken the numbers when a run
misses; defer the leftover and continue. Ratify or change them in an ADR before
they become `G5` release promises.

| Area | Provisional gate |
| --- | --- |
| Prompt | Cached p95 <= 10 ms; p99 <= 25 ms; provider deadline <= 50 ms; usable fallback within one 100 ms cycle |
| History write | Queue acknowledgement p95 < 2 ms and p99 < 5 ms; never wait unboundedly on SQLite |
| History query | Exact-prefix p95 < 10 ms and ranked/fuzzy p95 < 50 ms on 100k rows |
| Editing | Key-to-redraw p99 <= 16 ms; no external command on a cache-hit keystroke |
| Completion | Adapter overhead <= 5 ms over stock; 100% exact insertion parity on the accepted corpus |
| Highlighting | p99 <= 5 ms for 4 KiB incomplete input; removing styling yields exact source bytes |
| Git | Cache-hit p99 <= 1 ms with no Git process; bounded refresh visible by the next prompt or within 1 second |
| Ctrl+R | 100k-row filtering p95 <= 50 ms; cancel restores the original buffer; selection never executes |
| Terminal restoration | `stty -g` identical before/after signal, crash, cancel, and resize; sentinel command executes normally |
| Security | Provider-controlled ANSI/OSC/control input never reaches terminal control; all payloads, result counts, queues, caches, and subprocess output are bounded |

## Cross-cutting risks requiring evidence

- Standard Readline exposes `READLINE_LINE` during `bind -x` but no supported
  after-every-key decoration hook. Rebinding printable keys is a stop/reassess
  condition.
- Completion functions depend on live shell state and `compopt`; asynchronous or
  subprocess execution can change semantics.
- MBX1 is sequential and prompt-oriented. History RECORD uses MBX2 on the same
  coprocess. Interactive features may still need typed results, generation IDs,
  cancellation, and stale-response rejection as a later MBX2 revision, not an
  MBX1 change.
- A single sequential coprocess can suffer head-of-line blocking.
- Unicode scalar counts are not display widths. Combining characters, wide
  glyphs, wrapping, `SIGWINCH`, tmux, and SSH need PTY evidence.
- History captures command text that may contain secrets. Local storage,
  exclusions, retention, permissions, deletion, and no-log behavior are release
  contracts, not optional polish.
- Duration will be absent when timing is disabled; Phase 3 and ranking must accept
  nullable duration unless a safe explicit adapter is adopted.

## Deferred scope

Until the MVP gates pass, defer Python/Node/Docker/Kubernetes/cloud providers,
provider SDK/plugin ecosystem, command palette, directory frecency, contextual
error intelligence, command result blocks, cloud history, graphical UI, terminal
emulator work, AI assistance, and automatic command correction or execution.

## Change log

| Date (UTC) | Change |
| --- | --- |
| 2026-08-15 | Created the canonical roadmap from the architecture brief, current repository audit, SOLID refactor review, and explicit reassessment gates. |
| 2026-08-15 | Removed circular Phase 0/completion gates; split Phase 3A from full Phase 3; added privacy, packaging, durability, and future-phase deliverables after independent review. |
| 2026-08-15 | Added the post-refactor SOLID contract audit: response-envelope ownership, bounded/terminator-independent framing, port-contract evidence, fallback safety, and additive-flag parity. |
| 2026-08-15 | Started the bounded SOLID hardening checklist and paused PTY, history, and later roadmap work by explicit user direction. |
| 2026-08-15 | Completed the bounded SOLID hardening checklist: transport owns envelopes, MBX1 acquisition is capped across terminators, prompt adapters share one context/deadline/safety contract, Git refresh is capped/cached, and focused/canonical/release evidence is recorded. `FND-001`/`G0` remain validation; no later phase was started. |
| 2026-08-15 | Completed `PTY-001`: std-only POSIX PTY driver with bounded read/write, resize, signal, and termios/`stty` probes, plus foundation coverage for prompt lifecycle, helper failure, Ctrl+C, Ctrl+Z, resize, and terminal restoration. `FND-001`/`G0` remain validation. |
| 2026-08-15 | Completed the history groundwork slice: `HIST-002` PTY admission characterization (HISTCONTROL/HISTIGNORE/history-off/`history -s`/multiline folding/renumbering/exit flush), `RSH-004` multiline/width/resize PTY validation, `PRM-005` completion, expanded ADR 0005 to the full `G1` contract, drafted the `HIST-003` Phase 3A contract and `HIST-004` benchmark budgets. `G1` acceptance and `G0` baseline review remain open. |
| 2026-08-15 | Review fixes: corrected the not-implemented list after `RSH-004` completion (arbitrary key injection remains open), recorded the PTY driver macOS constants as `HRD-001` pre-work, removed dead driver API surface, and fixed the `visible_text` CSI/OSC terminator handling. |
| 2026-08-15 | Second review fixes: corrected the history-off `HISTCMD` evidence, clarified per-session writer topology, hardened parent PTY opening with `O_NOCTTY`, strengthened PS2/`history -a` regression evidence, and right-sized the exact near-limit Bash transport fixture budget. |
| 2026-08-15 | Accepted `G1` (ADR 0005) and approved the `HIST-003` Phase 3A contract; implemented the full UI-free history slice: bundled SQLite linkage with measured size evidence, storage schema v1 with WAL/`0700`/`0600`/retention, narrow ports, exclusions/disable/clear/delete controls, deterministic queries, MBX2 RECORD ingestion, and opt-in Bash observation with PTY end-to-end tests. `G2` evidence remains. |
| 2026-08-15 | Reconciled architecture, README, protocol, and gate status with that implemented slice: `G2` is `validation` rather than blocked by completed `G1`; MBX2 RECORD is specified as implemented; capture remains default-off until `G2` evidence. |
| 2026-08-15 | Started the `HIST-007` `.bash_history` invariance and recorder admission-parity PTY slice; plan in `docs/history-g2-invariance-plan.md`. `G2` budgets/contention remain. |
| 2026-08-15 | Completed the invariance/admission-parity PTY slice: paired `HISTFILE` comparisons for enable/disable/clear/delete/exit-flush, recorder parity with the `HIST-002` matrix, and recorder fixes for first-prompt skip, `history 1` parsing, and list-number drop keys (`M-026`–`M-028`). `G2` budgets/contention remain. |
| 2026-08-16 | Corrected ADR 0005/`history_number` to the `history 1` list number, not `HISTCMD`. Completed the `HIST-004` corpus, hostile-inertness, and 100k query-percentile slice (`docs/history-g2-corpus-plan.md`, `docs/benchmarks/2026-08-16-history-queries.md`). Writer batching and under-cap prune skips (`M-029`, `M-030`). `G2` contention, prompt-boundary write, many-match prefix, and extra permission checks remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as concurrent-writer contention cases 1–3 and 6; plan in `docs/history-g2-contention-plan.md`. Prompt-boundary write-ack, WAL crash/corrupt, many-match prefix, and extra permission checks remain. |
| 2026-08-16 | Completed concurrent-writer contention storage tests (C-1–C-4, C-6) and hardened concurrent migrate, read-only query opens, and writer lock retries (`M-032`) in `crates/cli/src/storage.rs`. Prompt-boundary write-ack, WAL crash/corrupt, many-match prefix, and extra permission checks remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as prompt-boundary write acknowledgement PTY; plan in `docs/history-g2-write-ack-plan.md`. WAL crash/corrupt, many-match prefix, and extra permission checks remain. |
| 2026-08-16 | Completed prompt-boundary write-ack PTY correctness (W-1–W-4) and recorded release percentile evidence (`docs/benchmarks/2026-08-16-history-write-ack.md`; budget miss on development WSL). Write-ack budget, WAL crash/corrupt, many-match prefix, and extra permission checks remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as WAL crash/corrupt (cases 4–5); plan in `docs/history-g2-wal-crash-plan.md`. Permission beyond mode bits, many-match prefix, and write-ack budget remain. Do not chase write-ack product-code optimization unless a test proves SQLite is on the prompt path. |
| 2026-08-16 | Completed WAL crash/corrupt storage tests (K-1–K-4) in `crates/cli/src/storage.rs` (`docs/history-g2-wal-crash-plan.md`). Write-ack budget, permission beyond mode bits, and many-match prefix remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as WAL/SHM `0600` plus never-more-permissive chmod; plan in `docs/history-g2-permission-plan.md`. Foreign-user open, many-match prefix, and write-ack budget remain. Do not chase write-ack product-code optimization unless a test proves SQLite is on the prompt path. |
| 2026-08-16 | Completed WAL/SHM `0600` and never-more-permissive storage tests (P-1–P-4) plus tighten-only chmod in `crates/cli/src/storage.rs` (`docs/history-g2-permission-plan.md`; `M-033`). Foreign-user open, many-match prefix, and write-ack budget remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as many-match exact-prefix latency (covering index, schema v2, ADR 0008); plan in `docs/history-g2-prefix-plan.md`. Foreign-user open and write-ack budget remain. Do not chase write-ack product-code optimization unless a test proves SQLite is on the prompt path. |
| 2026-08-16 | Completed many-match exact-prefix covering index (schema v2, ADR 0008, Q-A–Q-C in `crates/cli/src/storage.rs`; release percentiles in `docs/benchmarks/2026-08-16-history-prefix.md`; `docs/history-g2-prefix-plan.md`). Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as writer idle-flush for live reader visibility (PTY `wait_for_count` at `count=0`); plan in `docs/history-g2-idle-commit-plan.md`. Foreign-user open and write-ack budget remain. Do not change `WRITER_BATCH_SIZE` or ACK meaning. Do not fake `seteuid`. |
| 2026-08-16 | Completed writer idle-flush for live reader visibility (V-1–V-3 in `crates/cli/src/storage.rs`; PTY invariance `wait_for_count`; `docs/history-g2-idle-commit-plan.md`; `M-034`). Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Identified the next `HIST-007` slice as 100k-row v1→v2 migration (`HIST-004` case 8); plan in `docs/history-g2-migrate-100k-plan.md`. Foreign-user open and write-ack budget remain. Do not fake `seteuid`. Do not chase write-ack product-code optimization unless a test proves SQLite is on the prompt path. |
| 2026-08-16 | Completed 100k-row v1→v2 migration evidence (M-2 in `crates/cli/src/corpus.rs`; release wall time in `docs/benchmarks/2026-08-16-history-migrate.md`; `docs/history-g2-migrate-100k-plan.md`). Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Linked green GitHub Actions CI on `origin/main` at `5c077ce` (https://github.com/ishitvagoel/ColorBash/actions/runs/31932933113); completed `FND-001` and `BST-005` (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS, and `PRM-004` representative percentiles. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Completed Darwin PTY constant cfg-split pre-work for `HRD-001` (D-1–D-3 in `crates/pty/src/sys.rs`; `docs/hrd-001-darwin-pty-constants-plan.md`). Linux PTY tests stay green on WSL; full macOS matrix evidence still required. `HRD-001` and `G0` remain open. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Refreshed linked green GitHub Actions CI on `origin/main` to `2b98026` (https://github.com/ishitvagoel/ColorBash/actions/runs/31934458862); `FND-001` and `BST-005` remain complete (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS PTY run, and `PRM-004` representative percentiles. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Completed redirected-output color policy for direct `mbx prompt` (R-1–R-4 in `crates/cli/src/environment.rs`; `docs/prm-002-redirected-output-plan.md`; `M-009`). `PRM-002` width model remains discovery. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Refreshed linked green GitHub Actions CI on `origin/main` to `8872998` (https://github.com/ishitvagoel/ColorBash/actions/runs/31935135933); `FND-001` and `BST-005` remain complete (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS PTY run, and `PRM-004` representative percentiles. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Completed display-width path compaction helper for `PRM-002` (W-1–W-6 in `crates/cli/src/prompt.rs`; `docs/prm-002-width-plan.md`). `PRM-002` stays `discovery` for 16/256/truecolor and wrap-column PTY probes. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Completed color capability negotiation for `PRM-002` (T-1–T-6 in `crates/cli/src/environment.rs`, `crates/cli/src/prompt.rs`, `bash/config.bash`, `bash/fallback.bash`; `docs/prm-002-color-capability-plan.md`). `PRM-002` stays `discovery` for wrap-column PTY probes. Foreign-user open and write-ack budget remain. |
| 2026-08-16 | Refreshed linked green GitHub Actions CI on `origin/main` to `2ea3be4` (https://github.com/ishitvagoel/ColorBash/actions/runs/31935853161); `FND-001` and `BST-005` remain complete (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS PTY run, and `PRM-004` representative percentiles. Remaining `G2` is still foreign-user open and write-ack budget. |
| 2026-08-16 | Completed foreign-user open evidence for `HIST-007` (F-1–F-4 in `crates/cli/src/storage.rs`; `docs/history-g2-foreign-user-plan.md`; `sudo -n -u nobody` uid 65534). `G2` and `HIST-007` stay `validation` for write-ack budget. `PRM-002` stays `discovery` for wrap-column PTY probes. |
| 2026-08-16 | Cloud remeasure of prompt-boundary write-ack percentiles (`docs/history-g2-write-ack-cloud-plan.md`; `docs/benchmarks/2026-08-16-history-write-ack-cloud.md`): p50=2412, p95=2546, p99=2752 µs — p95 misses the provisional budget. `G2` and `HIST-007` stay `validation`; write-ack budget remains. No product-code changes. |
| 2026-08-16 | Refreshed linked green GitHub Actions CI on `origin/main` to `2a829ba` (https://github.com/ishitvagoel/ColorBash/actions/runs/31936945807); `FND-001` and `BST-005` remain complete (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS PTY run, and `PRM-004` representative percentiles. Write-ack budget remains. |
| 2026-08-16 | Completed non-DSR wrap-column PTY discovery for `PRM-002` (W-C-1–W-C-4 in `crates/pty/tests/multiline_width.rs`; `docs/prm-002-wrap-column-plan.md`; CPR/DSR waits forbidden on raw PTY). `PRM-002` moves to `validation` for representative `PRM-004` percentiles. `G0` stays `validation`. `G2` / `HIST-007` stay `validation` for write-ack budget. |
| 2026-08-16 | Refreshed linked green GitHub Actions CI on `origin/main` to `da019de` (https://github.com/ishitvagoel/ColorBash/actions/runs/31937322390); `FND-001` and `BST-005` remain complete (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS PTY run, and `PRM-004` representative percentiles. Write-ack budget remains. |
| 2026-08-16 | Refreshed linked green GitHub Actions CI on `origin/main` to `8c8dad2` (https://github.com/ishitvagoel/ColorBash/actions/runs/31937499009); `FND-001` and `BST-005` remain complete (`docs/fnd-001-ci-plan.md`). `G0` validation remains open for platform matrix, `HRD-001` macOS PTY run, and `PRM-004` representative percentiles. Write-ack budget remains. |
| 2026-08-16 | Deferred the prompt-boundary write-ack percentile leftover and marked `G2` / `HIST-007` complete (`docs/history-g2-write-ack-deferral.md`). W-1–W-4 correctness remains; WSL and cloud p95 misses are preserved; the 2 ms / 5 ms budget is not weakened and is not recorded as met. Capture stays default-off. |
| 2026-08-16 | Identified the next `G0` / `PRM-004` slice as fallback and Git-disabled prompt percentiles; plan in `docs/prm-004-fallback-plan.md`. Do not invent a representative dirty/large repo. Do not mark `PRM-004` or `G0` complete. |
| 2026-08-16 | Accepted timing-deferral policy (`docs/latency-budget-deferral.md`): unmet percentile targets no longer block development. Marked `G0` complete; `PRM-004` `deferred`; `EDT-001` `ready` (`docs/edt-001-bind-x-plan.md`). `HRD-001` remains release-matrix work. |
| 2026-08-16 | Completed `EDT-001` E-1–E-4 non-destructive `bind -x` insertion prototype (`bash/editor.bash`, `crates/pty/tests/editor_bind_x.rs`). `EDT-001` moves to `validation`; `G3` stays `discovery` until the remaining matrix bullets have evidence. Default chord `\C-x\C-y`; `MBX_EDITOR_OVERRIDE=1` opts into overwrite. |
| 2026-08-16 | Completed `EDT-001` G3 matrix M-1–M-4 (`docs/edt-001-g3-matrix-plan.md`): vi-insert `bind -x`, bracketed paste, resize after insert, Ctrl+Z then insert (`bash/editor.bash`, `crates/pty/tests/editor_bind_x.rs`). `G3` stays `discovery`; exact-byte / quoting / multiline insert remains. |
| 2026-08-16 | Completed `EDT-001` exact-byte / quoting / multiline insert B-1–B-4 (`docs/edt-001-exact-bytes-plan.md`, `crates/pty/tests/editor_bind_x.rs`). B-5 insert-time redraw note recorded; `G3` moves to `validation`; continuous decoration remains unproven. Do not mark `G3` or `EDT-001` complete. |
| 2026-08-16 | Completed `COMP-001` non-popup stock-completion adapter harness H-1–H-4 (`docs/comp-001-harness-plan.md`; `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs`). `COMP-001` moves to `validation`; `G4` stays `discovery`. Do not mark `G4` or `COMP-001` complete. |
| 2026-08-16 | Completed `COMP-002` core exact-insertion parity P-1–P-4 (`docs/comp-002-parity-plan.md`; `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs`). `COMP-002` moves to `validation`; `G4` stays `discovery`. Leftover matrix and latency budget remain. |
| 2026-08-16 | Gated completion test fixtures behind `MBX_COMP_FIXTURES=1` and added inspect-before-wrap (`_mbx_comp_wrap_existing_f`; F-1–F-4). `COMP-002` stays `validation`; `G4` stays `discovery`. Leftover matrix remains. |
| 2026-08-16 | Specified `COMP-002` leftover insertion matrix L-1–L-4 in `docs/comp-002-leftover-matrix-plan.md`. `--` / nested / slow fallthrough / latency remain a later leftover. `G4` stays `discovery`. |
| 2026-08-16 | Completed `COMP-002` leftover insertion matrix L-1–L-4 (`docs/comp-002-leftover-matrix-plan.md`; `crates/pty/tests/completion_harness.rs`). `COMP-002` stays `validation`; `G4` stays `discovery`. Second leftover (`--`, nested, slow fallthrough, latency) remains. |
| 2026-08-16 | Specified `COMP-002` `--` and nested insertion N-1–N-2 in `docs/comp-002-dash-nested-plan.md`. Slow/stateful fallthrough and latency remain a later leftover. `G4` stays `discovery`. |
| 2026-08-16 | Completed `COMP-002` `--` and nested insertion N-1–N-2 (`docs/comp-002-dash-nested-plan.md`; `crates/pty/tests/completion_harness.rs`). N-2 observation uses `echo $(printf ...)` because `:` captures printf stdout in substitution. `COMP-002` stays `validation`; `G4` stays `discovery`. Slow/stateful fallthrough remains. |
| 2026-08-16 | Specified `COMP-002` slow/stateful wrap fallthrough S-1–S-4 in `docs/comp-002-fallthrough-plan.md`. Adapter latency stays `deferred`. `G4` stays `discovery`. |
| 2026-08-16 | Completed `COMP-002` slow/stateful wrap fallthrough S-1–S-4 (`docs/comp-002-fallthrough-plan.md`; `crates/pty/tests/completion_harness.rs`). `COMP-002` stays `validation`; `G4` stays `discovery`. Deferred 5 ms budget remains. |
| 2026-08-16 | Specified `G4` decision inventory in `docs/g4-decision-plan.md`. Functional COMP-002 cases are recorded; 5 ms leftover stays `deferred`. Do not mark `G4` complete. |
| 2026-08-16 | Completed `G4` decision inventory (`docs/g4-decision-plan.md`; `docs/latency-budget-deferral.md`). `G4` moves to `validation`; 5 ms leftover stays `deferred`. `COMP-001` / `COMP-002` stay `validation`. Do not start `COMP-003` or popup. |
| 2026-08-16 | Specified `G3` decision inventory in `docs/g3-decision-plan.md`. Continuous decoration stays unproven. Do not mark `G3` complete. Do not start ghost, popup, or `COMP-003`. |
| 2026-08-16 | Completed `G3` decision inventory (`docs/g3-decision-plan.md`). `G3` stays `validation`; continuous decoration stays unproven. Do not start ghost, popup, or `COMP-003`. |
| 2026-08-16 | Specified `PRM-006` duration policy decision in `docs/prm-006-duration-plan.md`. Remain opt-in; do not compose `DEBUG`. Do not start ghost, popup, or `COMP-003`. |
| 2026-08-16 | Completed `PRM-006` duration policy decision (`docs/prm-006-duration-plan.md`; `tests/bash/smoke.bash` D-1–D-3). Remain opt-in; do not compose `DEBUG`. `PRM-006` moves to `validation`. Do not start ghost, popup, or `COMP-003`. |
| 2026-08-16 | Specified `G4` gate-close decision in `docs/g4-gate-close-plan.md`. Functional parity recorded; 5 ms stays `deferred`. Do not start `COMP-003` or popup. |
| 2026-08-16 | Completed `G4` gate close (`docs/g4-gate-close-plan.md`; `docs/g4-decision-plan.md`). `G4` moves to `complete`; 5 ms leftover stays `deferred`. `COMP-001` / `COMP-002` stay `validation`. `COMP-003` unblocked for planning. Do not start `COMP-003` or popup. |
| 2026-08-16 | Specified `COMP-003` typed metadata slice in `docs/comp-003-metadata-plan.md`. K-1–K-4; no ranking or popup. |
| 2026-08-16 | Completed `COMP-003` typed metadata K-1–K-4 (`docs/comp-003-metadata-plan.md`; `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs`). `COMP-003` moves to `validation`; ranking leftover remains. Do not start popup or ghost. |
| 2026-08-16 | Specified `COMP-003` bounded ranking slice in `docs/comp-003-ranking-plan.md`. R-1–R-4; no popup. |
| 2026-08-16 | Completed `COMP-003` bounded ranking R-1–R-4 (`docs/comp-003-ranking-plan.md`; `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs`). `COMP-003` moves to `complete`. Do not start popup or ghost. |
| 2026-08-16 | Specified `G3` gate-close decision in `docs/g3-gate-close-plan.md`. Explicit `bind -x` evidence recorded; continuous decoration stays unproven. Do not start popup or ghost. |
| 2026-08-16 | Completed `G3` gate close (`docs/g3-gate-close-plan.md`; `docs/g3-decision-plan.md`). `G3` and `EDT-001` move to `complete`. Continuous decoration leftover still blocks ghost / highlighting. `COMP-004` unblocked for planning. Do not start popup or ghost. |
