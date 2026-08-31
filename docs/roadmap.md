# MBX delivery roadmap

> Canonical status: this is the single living source for delivery order, current
> status, dependencies, gates, and immediate next work. The originating product
> brief remains `CODEX_MODERN_BASH_ARCHITECTURE.md`; its checkboxes describe the
> intended program and are not a status tracker.

- Last reviewed: 2026-08-31 UTC
- Current milestone: Strategy A MVP is `complete` on Linux (`G5` 2026-08-27); Phase 6 `complete` (ADR 0015 preview-row highlighting; `HLT-003` p99 `deferred`); Phase 9 `complete`; macOS `HRD-001` `deferred` (ADR 0012); `HRD-003` `deferred`; `COMP-004` overlay `complete` (type-to-filter GUI `deferred`)
- Active workstream: `REL-001` first `v*` tag (maintainer-gated); G5 revisit macOS PTY; dim paint; percentile benches `deferred`
- Next decision gate: G5 revisit (macOS matrix, `HLT-003` p99, `HRD-003`, dim paint). ADR 0015 closed `M-064`; `COMP-004` width guard recorded
- Editor-facing work: opt-in ghost suffix is on main (ADR 0010). Async QUERY with stale-generation skip is recorded (ADR 0011). Explicit history-search insert (`\C-xh`), cycling, restore (`\C-xl`), cwd preference, and opt-in failed empty-line insert are recorded (ADR 0009). Opt-in syntax highlighting (`MBX_HIGHLIGHT=1`, ADR 0015 preview row) and completion overlay (`MBX_COMP_OVERLAY=1`) are `complete` aside from deferred leftovers. Dim paint and type-to-filter overlays are `deferred` from this MVP (G5 revisit)
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

The "Change log" section at the end of this file keeps only its most recent
entries; the complete history lives in
[`docs/archive/roadmap-history.md`](archive/roadmap-history.md). Append a new
entry to both, in the same change, most-recent last — never edit or reorder a
past entry in either file.

Continuous after-every-key decoration is unproven (ADR 0003 B-5). That leftover
**defers** live highlighting and GUI-like overlays; it must not mark Strategy A
explicit `bind -x` or suffix-in-buffer features `blocked`. A missing ADR is the
next slice, not an indefinite wait.

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

The repository is a foundation prototype plus an opt-in history sidecar and
Strategy A ghost suffix, not an overlay MVP. A green GitHub Actions CI run is
linked for `FND-001`
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
- an opt-in SQLite history sidecar with schema v3 (forward-only from v1/v2), WAL, `0700`/`0600`
  permissions, retention, exclusions, path/count/clear/delete controls, and
  deterministic recent/prefix/cwd/repo queries plus bounded fuzzy ranking;
  capture remains off unless `MBX_HISTORY=1`; nullable `repo_root`/`repo_branch`
  are writer-enriched from absolute `start_cwd`;
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

- dim after-every-key paint or a type-to-filter Ctrl+R overlay (G5 revisit);
- `HLT-003` highlight p99 (hostile/PTY slices are recorded; percentiles
  `deferred`);
- the release platform matrix (`HRD-001` macOS `deferred` per ADR 0012; Linux
  nested/SSH/login/vim/tmux PTY recorded), or remaining G5 pairwise combinations
  on macOS; or
- prompt-boundary write-ack percentile budget (correctness recorded; p95 miss
  deferred from `G2` — `docs/history-g2-write-ack-deferral.md`).

Opt-in inline ghost is implemented (ADR 0010). `HIST-010` / `GIT-003` CLI
filters (`search repo` / `search branch` / `search failed`) are on main.

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
                         G2 + G3 ── Strategy A Ctrl+R ───┤
                         G2 + G3 ── ghost suggestions ───┤
                         G3 + G4 ── ranked completion ───┤
                              G3 ── highlighting last ───┤  (deferred; no paint hook)
                                                        │
                         Strategy A MVP exits ── G5 release hardening
```

Phase numbers preserve the original brief. Actual execution order follows the
dependencies above. Phase 8 is a Strategy A explicit `bind -x` search action,
not a type-to-filter overlay. Highlighting and GUI popups stay last and are
`deferred` until a decoration hook or a new ADR exists.

## Gates

### G0 — Foundation stability

Status: `complete` (2026-08-16)

Functional foundation evidence is recorded. Remaining prompt percentile matrix
work is `deferred` and does not block product development
(`docs/latency-budget-deferral.md`). macOS `HRD-001` is `deferred` (ADR 0012).

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

`G2` is the Phase 3A gate. It permits history-driven editor experiments.
Full Phase 3 also required fuzzy ranking (`HIST-009`) and repository context
(`HIST-010`); both are complete.
It requires `HIST-002` through `HIST-008` plus `HIST-011`, `HIST-012`, and
`HIST-013`; `HIST-009` and `HIST-010` are complete.

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
  (B-5). Continuous after-every-key decoration stays unproven and **defers**
  highlighting and GUI overlays (`docs/g3-gate-close-plan.md`; ADR 0003). It
  does not block Strategy A `bind -x` or suffix-in-buffer features. Opt-in
  inline ghost is on main (ADR 0010).

### G4 — Completion parity

Status: `complete` (2026-08-16)

The non-popup `COMP-001`/`COMP-002` adapter experiment produced this gate. It may
run alongside `G3`, using the shared PTY harness. A GUI completion overlay is
`deferred` (no decoration hook; `docs/comp-004-popup-plan.md`). Ranked-accept
(`\C-x\C-a`) is on main; ranked-cycle `\C-xn` / `\C-xp` is recorded.
`COMP-005` Strategy A insert/fallthrough is `complete`. Overlay leftover stays
`COMP-004` `discovery`. Passed when
file completion and at least one existing `-F` completion function preserve
stock Bash behavior for:

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

Status: `complete` for Strategy A MVP on Linux (2026-08-27)

`docs/g5-strategy-a-close-plan.md` maps every non-deferred `G5` / Phase 9 claim
to tests on this tree. Linux `HRD-001` L-1–L-5, `HRD-002`, and `HRD-004` are
recorded. macOS `HRD-001` is **`deferred`** (ADR 0012), not `blocked`.
`HRD-003` percentiles are `deferred` (`docs/latency-budget-deferral.md`).

`HLT-003` p99, type-to-filter GUI overlay, and dim-paint work remain
`deferred` from this Strategy A MVP with owner **G5 revisit**. Do not
delete those IDs. Unmet percentile targets stay `deferred` and are not a
pass/fail requirement for this close. Preview-row highlighting and the
COLUMNS-clamped completion overlay are `complete` (ADR 0015; M-065).

## Phase summary

| Phase | Name | Status | Principal unfinished condition |
| ---: | --- | --- | --- |
| 0 | Research / architecture | `complete` | `G0` complete; macOS `HRD-001` `deferred` (ADR 0012) |
| 1 | Bootstrap | `complete` | CI linked; `BST-002`–`BST-004` complete; broader lifecycle tracing `deferred`; Linux platform matrix recorded |
| 2 | Prompt | `complete` | `PRM-001`/`PRM-009` complete; capability/width/wrap recorded; `PRM-004` percentiles `deferred` |
| 3 | History | `complete` | Phase 3A / `G2` complete; `HIST-009` and `HIST-010` complete; write-ack percentiles `deferred` |
| 4 | Ghost suggestions | `complete` | ADR 0010 suffix; ADR 0011 QUERY + generation skip + overlapping delayed-RESULT PTY; `GHST-004` functional PTY recorded; dim paint `deferred`; latency percentiles `deferred` |
| 5 | Completion | `complete` | Strategy A insert/fallthrough (`COMP-005`); `G4` / `COMP-001`–`COMP-003` / `GIT-004` complete; ranked-cycle `\C-xn` / `\C-xp`; `COMP-004` overlay `complete` (M-065 reservation; COLUMNS-1 width guard; type-to-filter GUI `deferred`) |
| 6 | Syntax highlighting | `complete` | ADR 0013/0014/0015; `HLT-004` coprocess `complete`; `HLT-002` preview-row PTY; `HLT-003` hostile slices 1–2; p99 `deferred` |
| 7 | Git/provider expansion | `complete` | MVP exits `GIT-002` / `GIT-004` (`docs/git-phase7-mvp-close-plan.md`); `GIT-005` SDK `deferred`; upstream/remotes/tags unauthorized |
| 8 | Enhanced Ctrl+R | `complete` | `SRCH-001`–`SRCH-003` complete (ADR 0009); cwd/signal/opt-in failed/opt-in repo insert recorded (`docs/srch-003-repo-filter-plan.md`); 100k interactive leftover `deferred`; overlay `deferred` |
| 9 | Release hardening | `complete` | `HRD-002` and `HRD-004` complete; Linux `HRD-001` L-1–L-5 recorded; macOS `HRD-001` `deferred` (ADR 0012); `HRD-003` `deferred`; `G5` closed (`docs/g5-strategy-a-close-plan.md`) |

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
| `BST-001` | Rust workspace, Bash loader, and development setup | `complete` | `Cargo.toml`, `bash/init.bash`, `scripts/dev-setup.bash`, `scripts/install.bash` |
| `BST-002` | Interactive guard, idempotence, status preservation, and fallback | `complete` | `docs/bst-prm-g0-leftover-close-plan.md` I-1–I-5; `tests/bash/smoke.bash`; `crates/pty/tests/foundation.rs`; platform matrix leftover is `HRD-001` |
| `BST-003` | MBX1 coprocess and per-call adapters | `complete` | `docs/bst-prm-g0-leftover-close-plan.md` A-1–A-2; `tests/bash/modules.bash`; `tests/integration/protocol.bash`; PTY helper-crash; platform matrix leftover is `HRD-001` |
| `BST-004` | Debug/trace logging without command text | `complete` | `docs/bst-prm-g0-leftover-close-plan.md` T-1; `crates/cli/src/telemetry.rs`; typed diagnostics omit command text; `MBX_DBG` forbidden; broader lifecycle tracing `deferred` |
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
| `PRM-001` | Path, Git, status, duration, SSH, production, icon, and theme segments | `complete` | `docs/bst-prm-g0-leftover-close-plan.md` S-1–S-5; `crates/cli/src/prompt.rs`; `tests/bash/modules.bash` fallback parity |
| `PRM-002` | Capability, redirected-output, visible-width, and resize model | `complete` | redirected-output color policy recorded (`docs/prm-002-redirected-output-plan.md`; `crates/cli/src/environment.rs`); display-width path compaction helper recorded (`docs/prm-002-width-plan.md`; `crates/cli/src/prompt.rs`); color capability (16/256/truecolor) recorded (`docs/prm-002-color-capability-plan.md`; `crates/cli/src/environment.rs`, `crates/cli/src/prompt.rs`, `bash/config.bash`, `bash/fallback.bash`); non-DSR wrap-column PTY usability recorded (`docs/prm-002-wrap-column-plan.md`; `crates/pty/tests/multiline_width.rs`) |
| `PRM-003` | End-to-end deadline, capped Git acquisition, and warm cache | `complete` | one Bash deadline, capped 50-ms Git refresh, 128-entry/1-s cache, failure tests, and `docs/benchmarks/2026-08-15-solid-hardening.md` |
| `PRM-004` | Full prompt p50/p95/p99 benchmark matrix | `deferred` | controlled warm-Git case recorded; remaining matrix deferred (`docs/latency-budget-deferral.md`) |
| `PRM-005` | Real PTY prompt, wrap, signal, resize, and restoration tests | `complete` | lifecycle, helper failure, Ctrl+C/Z, resize, `stty -g`, multiline, narrow wrap, and wide-glyph coverage |
| `PRM-006` | Decide opt-in duration policy or explicit preexec adapter | `complete` | `docs/prm-006-duration-plan.md`; D-1–D-4 in `tests/bash/smoke.bash`; remain opt-in; do not compose `DEBUG`; `G3`/`G4` complete |
| `PRM-007` | Give native and fallback adapters one explicit input/safety contract and shared hostile-state corpus | `complete` | explicit four-field context, shared C0/DEL/expansion corpus, production precedence, and SSH-only test |
| `PRM-008` | Preserve raw additive prompt flags across coprocess, per-call, and fallback paths | `complete` | raw `--flags` CLI boundary plus coprocess/per-call/fallback unknown-bit tests |
| `PRM-009` | Reassess semantic composition versus typed PS1 encoding and validated theme styles | `complete` | `docs/bst-prm-g0-leftover-close-plan.md` D-1; keep semantic roles → theme SGR; `PRM-002`/`PRM-007` do not justify a typed PS1 encoding |

Exit condition: prompt requirements of `G0`.

### Phase 3 — History

Status: `complete` for the UI-free Phase 3A / `G2` slice (2026-08-16).
`HIST-009` fuzzy ranking and `HIST-010` repository context are `complete`.
Write-ack percentiles are `deferred` (`docs/history-g2-write-ack-deferral.md`).

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
| `HIST-008` | Add recent, exact-prefix, cwd queries and deterministic ranking | `complete` | `mbx history search recent|prefix|cwd|failed` with bounded limits and NOCASE prefix index; failed leftover in `docs/hist-010-cli-filters-plan.md` |
| `HIST-009` | Add bounded fuzzy ranking | `complete` | `docs/hist-009-fuzzy-plan.md`; `mbx history search fuzzy`; scores over the most recent 256 rows; `crates/cli/src/history.rs`, `storage.rs` |
| `HIST-010` | Add repository context | `complete` | `docs/hist-010-git-003-plan.md`; schema v3; writer enrich from `start_cwd`; `mbx history search repo` / `search branch`; PTY `admitted_commands_record_repository_root_and_are_searchable_by_repo`; CLI leftovers in `docs/hist-010-cli-filters-plan.md` |

Exit conditions: `G1` before capture can be enabled; `G2` before history-driven
editor UI; full Phase 3 completion additionally requires `HIST-009` and
`HIST-010` (both complete). Deferring either beyond the MVP requires an accepted scope/ADR
decision and corresponding reconciliation of the authoritative product brief;
the roadmap cannot make that scope change by itself.

### Phase 4 — Ghost suggestions

Status: `complete` for Strategy A suffix ghost (2026-08-25). Word-accept,
cycling, remaining printables, vi-insert, Left dismiss, Home/Up/Down/backward-word,
and kill-ring isolation are recorded. Dim after-every-key paint is `deferred`
(not a Phase 4 exit). Async lookup is `GHST-001` `complete` (ADR 0011 + QUERY
wire + overlapping delayed-RESULT PTY). `GHST-004` functional editing and safety
PTY evidence is recorded; latency percentiles stay `deferred`.

Implement asynchronous ranked-history lookup with generation
IDs, stale-result rejection, inline rendering, full/word acceptance, cycling, and
multiline/resize behavior. Acceptance must preserve exact bytes/cursor position,
never execute the suggestion, and perform no external command on a cache-hit
keystroke.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `GHST-001` | Async ranked query with generation IDs and cancellation | `complete` | ADR 0011; QUERY/RESULT/CANCEL wire; ghost coprocess QUERY + generation check (`docs/ghst-001-ghost-query-plan.md` W-1); overlapping delayed-RESULT PTY W-2; CANCEL-after-QUERY prompt W-4 (`crates/pty/tests/ghost.rs`) |
| `GHST-002` | Inline ghost rendering with stale-result rejection | `complete` | ADR 0010; G-1–G-6 in `bash/ghost.bash`, `crates/pty/tests/ghost.rs`, `tests/bash/modules.bash`; `docs/ghst-002-inline-ghost-plan.md`; remaining printables P-1–P-3 (`docs/ghst-002-printables-plan.md`); vi-insert V-1–V-3 (`docs/ghst-002-vi-insert-plan.md`); Left dismiss L-1–L-3 (`docs/ghst-002-left-motion-plan.md`); Home/Up/backward-word H-1/W-4/U-2 (`docs/ghst-002-home-up-motion-plan.md`); Down D-1–D-2 (`docs/ghst-002-down-motion-plan.md`); kill-ring isolation K-1–K-3 (`docs/ghst-002-kill-ring-plan.md`); Enter is a Readline delete-char + accept-line macro (M-041); helpers bind before printables and partial disarm clears the armed flag (M-044); dim paint is `deferred` (not a Phase 4 exit); async stale-rejection is `GHST-001` |
| `GHST-003` | Full/word acceptance and suggestion cycling | `complete` | Right/`\C-f` full accept in G-2; `\ef` / Ctrl-Right word-accept in W-1–W-3 (`docs/ghst-003-word-accept-plan.md`); `\C-x\C-n` / `\C-x\C-p` cycling in C-1–C-3 (`docs/ghst-003-cycle-plan.md`) |
| `GHST-004` | Multiline, resize, exact-byte, no-execution, and latency evidence | `complete` | R-1/Q-1/M-1 in `docs/ghst-004-multiline-resize-plan.md`; C-1/B-1 in `docs/ghst-004-no-execution-plan.md`; `crates/pty/tests/ghost.rs`; latency matrix `deferred` (`docs/latency-budget-deferral.md`) |

Exit condition: `GHST-004` functional editing and safety evidence. Latency
percentiles are `deferred` and do not block that exit.

### Phase 5 — Completion

Status: `complete` for Strategy A insert/fallthrough (`COMP-005`, 2026-08-25).
Popup policy is `complete` (`docs/comp-004-popup-plan.md` P-1–P-4). Strategy A
overlay slice is `validation` (ADR 0013; `docs/comp-004-overlay-plan.md`). Tab
stays stock. Ranked-accept is on main. Ranked-cycle defaults to `\C-xn` / `\C-xp`
so ghost `\C-x\C-n` / `\C-x\C-p` stay free.
`COMP-001` / `COMP-002` / `COMP-003` / `GIT-004` / `COMP-005` are complete.

First adapt stock Bash completion and prove exact insertion parity. Only then add
typed candidate metadata, bounded ranking, optional Strategy A cycle chords, and
Git candidates. Unsupported completion specifications must fall through without
mutating the line. Do not move completion functions into a subprocess unless
live-state and `compopt` parity are demonstrated. Do not start a GUI overlay.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `COMP-001` | Build a non-popup stock-completion adapter harness | `complete` | `docs/comp-001-harness-plan.md`; H-1–H-4; `G4` complete; 5 ms leftover `deferred` |
| `COMP-002` | Prove file and one `-F` function's exact insertion parity | `complete` | `docs/comp-002-parity-plan.md`; P-1–P-4, F-1–F-4, L-1–L-4, N-1–N-2, S-1–S-4; `docs/g4-gate-close-plan.md`; 5 ms leftover `deferred` |
| `COMP-003` | Add typed candidate metadata and bounded ranking | `complete` | `docs/comp-003-metadata-plan.md` K-1–K-4; `docs/comp-003-ranking-plan.md` R-1–R-4 in `bash/completion.bash`, `tests/bash/modules.bash`, `crates/pty/tests/completion_harness.rs` |
| `COMP-004` | Add popup navigation and terminal-safe rendering | `complete` | Popup policy P-1–P-4 (`docs/comp-004-popup-plan.md`); ranked-accept A-1–A-6; ranked-cycle C-1–C-6; overlay slice OV-1 + PTY (`docs/comp-004-overlay-plan.md`, ADR 0013). **`M-065` fixed 2026-08-30**. **Width guard 2026-08-31**: `_mbx_comp_overlay_format_row` clamps each overlay row to `COLUMNS-1` (SGR skipped; non-ASCII two columns) so a wide candidate cannot wrap onto an extra reserved row. Evidence: `crates/pty/tests/overlay_screen.rs` (`overlay_clamps_a_wide_row_so_it_does_not_wrap`, `overlay_near_the_bottom_of_a_short_terminal_leaves_the_prompt_intact`) and `tests/bash/modules.bash` OV-2/OV-3 plus the format-row clamp contract. Type-to-filter GUI menu remains `deferred` |
| `COMP-005` | Insert/fall through exactly and pass the parity/PTY matrix | `complete` | `docs/comp-005-strategy-a-close-plan.md`; G4/COMP-002 P-1–P-4, L-1–L-4, N-1–N-2, S-1–S-4; ranked-accept A-1–A-6; ranked-cycle C-1–C-6 (`\C-xn` / `\C-xp`); `GIT-004` kinds; overlay lives on `COMP-004` (`complete`; type-to-filter GUI `deferred`); 5 ms leftover `deferred` |

Exit condition: `G4` for the adapter slice; `COMP-005` for the Strategy A
completion feature. Overlay slice needed `HLT-003`-class hostile/latency
evidence before `COMP-004` could move to `complete`; that evidence-gathering
found `M-065`, a confirmed terminal-corruption defect, not just an unproven
claim. `M-065` was fixed on 2026-08-30 — the overlay reserves its rows with
IND before saving the cursor, and caps the draw at `LINES-2`. The COLUMNS-1
visible-width guard landed 2026-08-31 (`overlay_clamps_a_wide_row_so_it_does_not_wrap`).
Type-to-filter GUI menus remain `deferred` and do not block `COMP-004`
`complete` (same rule as other deferred leftovers on complete phases).

### Phase 6 — Syntax highlighting

Status: `complete` (ADR 0013/0014/0015). `READLINE_LINE` stays permanently
plain; the helper's styled copy paints on one reserved row below the prompt
(ADR 0015), so Readline never caret-renders `\001`/`\002` (`M-064` fixed).
Color is a tty-paint decision (`_mbx_highlight_color_flag`; `bind -x` stdout
is often a pipe). `HLT-003` p99 stays `deferred`
(`docs/latency-budget-deferral.md`) and does not block this close (G2/G4
precedent).

Define a tolerant token taxonomy only after Readline redraw feasibility is known.
The highlighter must accept incomplete Bash, never execute or expand input, bound
work by input size, classify dangerous text visually without blocking execution,
and strip back to the exact original bytes.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `HLT-001` | Define token taxonomy and tolerant incomplete-input lexer | `complete` | `docs/hlt-001-lexer-plan.md`; `crates/cli/src/highlight.rs`; `cargo test -p mbx highlight::` |
| `HLT-002` | Integrate terminal-safe styling without taking execution ownership | `complete` | ADR 0015; `bash/highlight.bash`; `tests/bash/modules.bash`; `crates/pty/tests/highlight.rs` (`highlight_preview_row_paints_sgr_below_an_intact_prompt`: Screen shows SGR copy on a row below an intact prompt; Enter executes exact plain bytes) |
| `HLT-003` | Pass exact-byte stripping, hostile-input, PTY, and latency gates | `complete` | `docs/hlt-003-hostile-gate-plan.md`; slices 1–2 S-1–S-4 and P-1–P-2 recorded; p99 `deferred` |
| `HLT-004` | Route HIGHLIGHT over the coprocess instead of forking per keystroke | `complete` | `docs/adr/0014-highlight-over-coprocess.md`; `crates/cli/src/highlight_service.rs`; `docs/protocol-mbx2.md` HIGHLIGHT/STYLED; `crates/pty/tests/highlight.rs` (`wire_highlight_forks_no_helper_process_per_keystroke`, `cli_fallback_highlight_does_fork_the_helper_per_keystroke`) |

Exit condition: `HLT-003` hostile/PTY gates plus `M-064` resolved. Met
2026-08-31 (ADR 0015). p99 remains `deferred`. Do not delete these IDs.

### Phase 7 — Git and provider expansion

Status: `complete` for the Strategy A MVP (`GIT-002` / `GIT-004`, 2026-08-25).
Prompt status plus history-scoped root/branch exist. Upstream/branches/remotes/tags
remain unauthorized. `GIT-005` stays `deferred` post-MVP.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `GIT-001` | Typed prompt repository-status provider | `complete` | `crates/cli/src/provider.rs`, ADR 0007, and provider substitution/degradation tests |
| `GIT-002` | Deadline, capped acquisition, TTL cache, refresh, invalidation | `complete` | ADR 0007, provider outcome/process/cache tests, and `docs/benchmarks/2026-08-15-solid-hardening.md` |
| `GIT-003` | Repository root/branch context, then upstream/branches/remotes/tags | `complete` | root/branch subset for `HIST-010` (`docs/hist-010-git-003-plan.md`); upstream/remotes/tags unauthorized |
| `GIT-004` | Structured completion metadata/ranking | `complete` | `docs/git-004-kinds-plan.md`; git/ref/flag/file kinds beside `COMPREPLY`; `mbx_comp_git` fixture; no Git subprocess |
| `GIT-005` | General provider capabilities/SDK | `deferred` | post-MVP evidence; ADR 0007 update required |

Do not add Python, Node, Docker, arbitrary executable plugins, or a generic SDK
until the MVP's concrete provider consumers establish the required contracts.

Exit conditions: `GIT-002` for the safe prompt-provider slice; `GIT-004` for the
MVP Git/completion slice. `GIT-005` remains explicitly post-MVP.

### Phase 8 — Enhanced Ctrl+R

Status: `complete` for Strategy A explicit `bind -x` search (2026-08-25; ADR
0009). A type-to-filter overlay remains `deferred`. Default insert is `\C-xh`;
restore is `\C-xl`.

Build a configurable explicit search action with cwd and useful status
metadata; bounded filtering; safe cancellation; exact insertion without
execution; and terminal restoration. Age/cwd/status columns stay overlay
`deferred`. CLI `search repo` / `search branch` / `search failed` exist on
main (`HIST-010`). Interactive empty-line `\C-xh` uses cwd, then recent;
`MBX_SEARCH_FAILED=1` prefers `search failed` first;
`MBX_SEARCH_REPO=1` prefers `search repo` at the root `mbx repo root`
resolves (`docs/srch-003-repo-filter-plan.md`).

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `SRCH-001` | Configurable bounded history-search action and result view | `complete` | insert S-1–S-7 and cycling V-1–V-4 in `bash/search.bash`, `crates/pty/tests/history_search.rs`, `tests/bash/modules.bash`; ADR 0009; `docs/srch-001-history-search-plan.md`; `docs/srch-001-result-view-plan.md`. Overlay is not required |
| `SRCH-002` | Cancel restoration and exact insertion without execution | `complete` | restore R-1–R-4 in `bash/search.bash`, `crates/pty/tests/history_search.rs`, `tests/bash/modules.bash`; ADR 0009; `docs/srch-002-cancel-restore-plan.md`. Overlay is not required |
| `SRCH-003` | Metadata filters, 100k-row latency, signal, and terminal-state evidence | `complete` | cwd empty-line C-1–C-4, prefix/fuzzy cwd, signal/terminal-state T-1–T-4, opt-in failed insert F-1–F-3, and opt-in repo insert R-1–R-3 in `bash/search.bash`, `crates/cli/src/cli.rs` (`mbx repo root`), `crates/pty/tests/history_search.rs`, `tests/bash/modules.bash`; `docs/srch-003-cwd-filter-plan.md`; `docs/srch-003-cwd-prefix-plan.md`; `docs/srch-003-signal-plan.md`; `docs/srch-003-failed-filter-plan.md`; `docs/srch-003-repo-filter-plan.md`. Overlay `deferred`; 100k interactive leftover `deferred`. CLI `search failed` / `search repo` / `search branch` are on main; interactive repo insert (`MBX_SEARCH_REPO=1`) is also on main |

Exit condition: `SRCH-003`. Overlay leftover stays `deferred`. 100k interactive
percentiles stay `deferred` (`docs/latency-budget-deferral.md`) and do not
block this Strategy A exit.

### Phase 9 — Release hardening

Status: `complete` for Strategy A MVP on Linux (2026-08-27). `HRD-002` and
`HRD-004` are complete. Linux `HRD-001` pairwise L-1–L-5 is recorded. macOS
`HRD-001` is **`deferred`** (ADR 0012). `G5` close evidence is in
`docs/g5-strategy-a-close-plan.md`.

The final matrix must include supported Bash 5.x releases; Linux, WSL, and macOS;
interactive/login/nested shells; emacs/vi modes; tmux and SSH; 16/256/true color;
Unicode and plain text; helper crashes; timeout/malformed IPC; Ctrl+C/Ctrl+Z;
resize and wrapping; background jobs; common prompt/preexec frameworks; and
representative fullscreen applications. Use pairwise coverage where exhaustive
combinations are impractical.

| ID | Deliverable | Status | Evidence or dependency |
| --- | --- | --- | --- |
| `HRD-001` | Supported Bash/OS/terminal pairwise PTY matrix | `complete` | Linux L-1–L-5 recorded (`docs/hrd-001-linux-pairwise-plan.md`; `crates/pty/tests/hrd001_linux.rs`); Darwin PTY constants D-1–D-3 recorded; macOS pairwise leg **`deferred`** (ADR 0012); Bash 5.0/5.1/5.2 (`ubuntu:20.04`/`22.04`/`24.04`) legs now run the three Bash suites in CI (`.github/workflows/ci.yml` `bash-matrix`); a manual `workflow_dispatch` macOS job exists so the deferred leg has somewhere to run once a host is available |
| `HRD-002` | Hostile input, protocol bounds, privacy, and no-execution audit | `complete` | `docs/hrd-002-hostile-audit-plan.md` H-1–H-11; C0/DEL insert gate on search/editor; ghost suffix gate; protocol/PS1/privacy/Git evidence; `G5` leftover tmux/SSH/fullscreen stays `HRD-001` |
| `HRD-003` | Release-mode end-to-end latency and resource evidence | `deferred` | existing warm-Git / history-query / write-ack records; remaining matrix `deferred` (`docs/latency-budget-deferral.md`); do not chase product-code latency |
| `HRD-004` | Install, upgrade, disable, removal, crash, and recovery evidence | `complete` | `docs/hrd-004-lifecycle-plan.md` L-1–L-6; setup/init never write `~/.bashrc`; helper crash and WAL recovery recorded; no package-manager installer |
| `DIAG-001` | `mbx doctor` diagnostic command (`CODEX_MODERN_BASH_ARCHITECTURE.md` §41) | `complete` | `mbx_doctor` in `bash/config.bash`: Bash version, interactivity/tty, color/locale/icon capability, helper path/version/live handshake, IPC mode, config resolution, per-feature keybinding-collision report with the matching `*_OVERRIDE` fix, ghost/highlight exclusion check, history store path/permissions/row count; module contracts D-1–D-3 in `tests/bash/modules.bash`; README §"Check it is installed" points here instead of per-feature manual recipes |
| `REL-001` | Prebuilt-binary release pipeline (GAP-1, `docs/repo-review-2026-08-29.md`) | `in-progress` | `.github/workflows/release.yml` plus `scripts/package-release.bash` (same tarball+checksum as the workflow Package step). Dry-run 2026-08-31 on `x86_64-unknown-linux-gnu`: tarball contained `mbx`, `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`; `sha256sum -c` matched. **No `v*` tag has been pushed**; cutting the first tag is a maintainer decision. `workflow_dispatch` is build-only (M-071). `scripts/install.bash` preferring a verified download over `cargo build` stays deferred until a real release exists |

Exit condition: `G5` after every non-deferred `HRD-*` item is complete.
macOS `HRD-001` is explicitly `deferred` (ADR 0012).

## Immediate next work

Strategy A MVP on Linux is `complete` (`G5` 2026-08-27). Capture stays
disabled by default. Unmet percentile leftovers are `deferred` and must not
block product slices (`docs/latency-budget-deferral.md`).

1. **G5 revisit** when a macOS host is available: run the `HRD-001` pairwise
   matrix per ADR 0012. Do not fake it on Linux.
2. **`REL-001`**: maintainer `workflow_dispatch` smoke of `release.yml`, then
   the first `v*` tag. Do not push a tag from an agent unless asked.
3. `HLT-003` p99 / `HRD-003` / `PRM-004` percentiles stay `deferred` unless
   an ADR ratifies new numbers or a functional prompt-path defect is proven.
4. `GIT-005` provider SDK stays post-MVP `deferred`.
5. Dim paint and type-to-filter overlays stay `deferred`.
6. Do not enable capture by default. Do not combine `MBX_GHOST=1` with
   `MBX_HIGHLIGHT=1`.

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
  after-every-key decoration hook. Strategy A self-insert wrapping (ADR 0010,
  ADR 0013) is the accepted workaround for ghost suffix and opt-in highlighting.
  Type-to-filter overlays and dim paint remain `deferred`. Rebinding printables to
  fake an overlay is a stop/reassess condition. Strategy A explicit `bind -x` and
  suffix-in-buffer features are not blocked by it.
- Completion functions depend on live shell state and `compopt`; asynchronous or
  subprocess execution can change semantics.
- MBX1 is sequential and prompt-oriented. History RECORD uses MBX2 on the same
  coprocess. ADR 0011 accepts interactive QUERY/RESULT/CANCEL with generation
  IDs and client stale rejection as an MBX2 extension; overlapping delayed
  RESULT skip is recorded (`GHST-001`). Do not overload MBX1.
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

Also `deferred` from this **Strategy A MVP** (owner G5 revisit; IDs kept):

- macOS `HRD-001` pairwise PTY matrix (ADR 0012);
- `HLT-003` highlight p99;
- dim after-every-key ghost paint;
- type-to-filter GUI completion / Ctrl+R overlay.

Do not leave those items `blocked` with no next action. Revisit at G5 or with
an accepted decoration/ownership ADR.


## Change log

Full history (138 entries as of 2026-08-31; the 126 present at the trim are
byte-identical to what was here before it) lives in
[`docs/archive/roadmap-history.md`](archive/roadmap-history.md). Append new
entries to *both* this table and that file, most-recent last, exactly as the
maintenance contract above requires; this table keeps only the most recent
entries for at-a-glance context.

| Date (UTC) | Change |
| --- | --- |
| 2026-08-27 | Accepted ADR 0012 macOS `HRD-001` deferral. Closed `G5` and Phase 9 for Strategy A MVP on Linux (`docs/g5-strategy-a-close-plan.md`). `HRD-001` Linux `complete`; macOS `deferred`. Overlay/highlighting/percentiles stay `deferred`. |
| 2026-08-27 | Accepted ADR 0013 opt-in continuous decoration. Implemented `MBX_HIGHLIGHT=1` (`bash/highlight.bash`, `mbx highlight`) and `MBX_COMP_OVERLAY=1` (`bash/completion.bash`). `HLT-001`/`HLT-002` and `COMP-004` overlay slice move to `validation`; `HLT-003` hostile/latency gates stay `deferred`. |
| 2026-08-27 | Review close plan for ADR 0013 (`docs/hlt-comp-review-close-plan.md`). Highlight wrap is a no-op until H-1–H-6; overlay leftover is O-1–O-5. Do not mark `HLT-002` or `COMP-004` complete. |
| 2026-08-27 | Implemented ADR 0013 review-close slices 1–3 (`bash/highlight.bash`, `bash/completion.bash`, module + PTY asserts H-1–H-6, O-1–O-5, M-1). `HLT-002` and `COMP-004` overlay stay `validation`; `HLT-003` stays `deferred`. |
| 2026-08-27 | Opened `HLT-003` hostile corpus slice (`docs/hlt-003-hostile-gate-plan.md`): UTF-8 lexer advance fix, Rust/Bash strip round-trip, PTY hostile execute-plain and C0 refusal. `HLT-003` moves to `in-progress`; p99 stays `deferred`. |
| 2026-08-27 | Recorded `HLT-003` slices 1–2: hostile corpus strip round-trip (S-1–S-4), PTY hostile execute-plain (P-1), module C0 refusal (P-2). UTF-8-safe lexer/strip in `highlight.rs` and `bash/highlight.bash`. `HLT-003` stays `in-progress`; p99 `deferred`. |
| 2026-08-28 | Added `scripts/install.bash` profiles (`comfort` / `highlight` / `prompt`), optional `~/.config/mbx/config.bash`, `mbx_status`, and `MBX_COMP_WRAP`. Default install and `source init.bash` still never write `~/.bashrc`; `--bashrc` is an explicit managed block. Capture stays off without a profile or `MBX_HISTORY=1`. Do not mark `HLT-002`, `COMP-004`, Phase 6, or `HLT-003` complete. |
| 2026-08-28 | Added `scripts/configure.bash` interactive option menu (`install.bash --interactive`, `mbx_configure`). `--answers FILE` covers the same keys without a TTY. Ghost+highlight still cannot combine; persist-in-bashrc stays opt-in. Do not mark `HLT-002`, `COMP-004`, Phase 6, or `HLT-003` complete. |
| 2026-08-28 | Review fixes for install/configure: isolate `XDG_CONFIG_HOME` in smoke HOME tests (M-057); `--bashrc` follows a HOME-local bashrc symlink and refuses targets outside `$HOME`; wrap copies `-P`/`-S`/`-X` as well as `-o`. Do not mark `HLT-002`, `COMP-004`, Phase 6, or `HLT-003` complete. |
| 2026-08-28 | Configure re-entry loads the saved file (opening choice 4 / `--from-config`; `mbx_configure` passes `--from-config`). `--build` runs `cargo build --release --workspace`. `mbx_status` prints duration, persist-bashrc, and helper executable/missing. Do not mark `HLT-002`, `COMP-004`, Phase 6, or `HLT-003` complete. |
| 2026-08-29 | Repository review (`docs/repo-review-2026-08-29.md`) plus its Track 0 fixes: `unreadable_store_fails_closed_without_widening` no longer assumes a non-privileged caller (M-060), so `bash tests/run.bash` now completes as root; `tests/integration/protocol.bash` resolves a relative binary argument before the case that changes directory (M-061); CI (`.github/workflows/ci.yml`) gained an MSRV 1.85.0 job, a release-profile build job, Bash 5.0/5.1/5.2 legs for `HRD-001`, and a manual macOS `workflow_dispatch` job. |
| 2026-08-29 | `HLT-004` (Track 1 of the review plan): accepted ADR 0014, routing `MBX_HIGHLIGHT=1`'s live refresh over the coprocess via a new independent `HighlightHandler` and MBX2 `HIGHLIGHT`/`STYLED` frame pair, structurally eliminating the per-keystroke helper-process fork (`crates/pty/tests/highlight.rs` proves zero non-serve `mbx` invocations while the coprocess is ready, contrasted with the CLI fallback). Also fixed `M-063` (a `[N] PID` job announcement leaking from `_mbx_engine_write`/`_mbx_engine_exchange` when called from a `bind -x` keystroke callback — affected ghost's existing wire path too; its own PTY suite got noticeably faster once fixed). While fixing highlight's color-detection bug (`M-062`, mitigated: the helper decided color from its own never-a-terminal stdout), found a deeper, previously-undiscovered defect (`M-064`, open): Readline caret-renders `\001`/`\002` inside `READLINE_LINE` rather than hiding them as it does in `PS1`, so the live interactive path has never rendered real color correctly. The interactive refresh deliberately keeps `color=0` until `M-064` is resolved; `HLT-002` moves to `blocked` and Phase 6 stays `validation` pending that work, not the previously-tracked percentile leftover. |
| 2026-08-29 | `DIAG-001` (Track 4 of the review plan): added `mbx_doctor`, the `mbx doctor` diagnostic command from `CODEX_MODERN_BASH_ARCHITECTURE.md` §41, which the roadmap had never carried an ID for. Reports and explains, with a fix line, every check `mbx_status` only summarized plus several it never covered (keybinding collisions per feature, history store permissions/health, live handshake). README's per-feature "Check it is installed" recipes now point to it instead of duplicating manual `bind -X`/`printf` snippets. |
| 2026-08-29 | Track 2 of the review plan: added a minimal VT screen model (`crates/pty/src/screen.rs`) so a PTY test can assert what a terminal actually shows, not just that a substring eventually appeared in the raw byte stream. Used it to write the failing case the overlay's terminal-safety claim never had: at a short terminal with the prompt a few lines down, showing an eight-candidate overlay scrolls the screen, which invalidates the overlay's `\e7`/`\e8` (DECSC/DECRC) absolute-position save and makes the following `\e[J` erase the prompt and all prior output from the wrong origin (`M-065`, open). A `SIGWINCH` while the overlay is visible is confirmed unaffected. `COMP-004` moves from `validation` to `blocked` — this was a confirmed defect the review found, not merely an unproven claim; a correct fix needs a cursor-row query this codebase does not have, or a different rendering strategy, so no fix is attempted in this change. The reproducing test stays `#[ignore]`-marked evidence (`crates/pty/tests/overlay_screen.rs`) rather than a permanently red canonical suite. |
| 2026-08-29 | Track 3 of the review plan: closed the last authorized `SRCH-003` gap. Added `mbx repo root [--cwd PATH]`, a thin CLI wrapper around the existing ADR 0007 Git adapter, so Bash can learn the current Git worktree root without ever calling `git` itself. `MBX_SEARCH_REPO=1` (`bash/search.bash`) resolves that root on an empty-line `\C-xh` and prefers `history search repo ROOT`, falling through to cwd/recent outside a worktree or when the repo has no rows (`docs/srch-003-repo-filter-plan.md`; ADR 0009 decision 4 extended). This also reconciles the roadmap with PR #48 (`ishitvagoel/ColorBash`), an equivalent, independently authored slice that had gone stale and conflicted against main; the three "interactive repo insert unauthorized" statements this depended on are corrected. PTY evidence: `empty_line_inserts_repo_when_opt_in` (a real `git init` worktree; a row recorded elsewhere in the same repository outranks a newer row recorded outside it) and `empty_line_repo_falls_back_when_not_in_a_repository`. |
| 2026-08-29 | `REL-001` (Track 4 of the review plan): added `.github/workflows/release.yml`, a `v*`-tag-triggered pipeline building `mbx` for `x86_64`/`aarch64` Linux on native runners and publishing a checksummed GitHub release, addressing `GAP-1` (no tag, release, or prebuilt binary existed). `in-progress`, not `complete`: it is untested end-to-end (no tag has been pushed) and cutting the first tag is a maintainer decision. `scripts/install.bash`'s download-preferring half is deliberately deferred until a real release exists to validate against. |
| 2026-08-29 | Track 4 of the review plan: split `README.md` into a short introduction (install, comfort profile, the six rules that always hold, feature map, `mbx doctor`) and `docs/reference.md` (the ten per-feature walkthroughs, the full environment-variable list, and automated test commands), unchanged in substance. README dropped from ~24 KB / 608 lines to ~8 KB / 144 lines; nothing was deleted, only relocated and cross-linked. |
| 2026-08-30 | Review fixes for the review-plan branch, each with a test confirmed to fail against the unfixed code: highlight's coprocess loop now skips a queued history `ACK` the way ghost's identical loop already did, instead of tearing down a healthy helper when `MBX_HIGHLIGHT=1` and `MBX_HISTORY=1` share the one coprocess (`M-066`); `_mbx_search_repo_root` gates on the helper's exit status rather than trusting a possibly-partial first line from a killed child (`M-067`); added the `LICENSE-MIT`/`LICENSE-APACHE` texts that `Cargo.toml` has always declared and `release.yml` packaged, and dropped the error suppression that would have shipped a tarball without them (`M-068`). Also resynchronized this change log with `docs/archive/roadmap-history.md`, which was two entries behind the contract that the same commit introduced. No status values change. |
| 2026-08-30 | Fixed `M-069`: three provider tests that drive the real `git` binary were implicitly asserting the host can fork and exec `git` twice inside the product's hard 50 ms `MAX_GIT_DEADLINE` clamp. CI proved this machine-dependent — `context_returns_root_and_branch_for_a_worktree` timed out in the stable job and passed in the MSRV job on one identical commit. Added a `retry_while_timed_out` test helper scoped to `ProviderErrorKind::Timeout` alone, with its own contract test showing any other error kind and any wrong value still fail on the first attempt. The product deadline and its clamp invariant are unchanged. |
| 2026-08-30 | Review-feedback fixes on PR #52. `mbx doctor` now reports all ten chords MBX installs rather than only the three opt-in features, attributes a declined chord to a tty only for the two features that gate on one (and asks about stdin, as those installers do), and fails on a history store whose path resolves but whose row count does not (`M-070`). `release.yml` gates its publish job on `refs/tags/v*`, so a `workflow_dispatch` smoke run can no longer publish a release and an unintended tag from a branch (`M-071`). The Bash 5.0 CI leg this branch added caught a near-limit render-deadline assertion that was really a benchmark of the host's Bash build: measured against a from-source Bash 5.0, elapsed minus timeout is a flat ~121ms on 5.0 and ~32ms on 5.2 at every timeout tried, so the deadline is honored on both and only a fixed per-version cost differed. Replaced with a differential measurement across two render timeouts, which cancels that cost and asserts what the case is actually for (`M-072`). All three Bash suites now pass on a real Bash 5.0 build. |
| 2026-08-30 | Fixed `M-073`, found by the new Bash 5.0 CI leg: `tests/bash/corpus.bash` wrote its `MBX_TEST:` marker prefix as a literal in its own source, so the smoke suite's `grep -o` captured echoed input lines alongside real program output. That made the compatibility comparison silently require MBX to leave Bash's input echo byte-identical — which changing `PS1`/`PS2` and Readline state makes impossible by design — and made the result depend on the Readline build. Ubuntu 20.04's Bash 5.0 failed it while every real corpus result matched exactly. The prefix now lives in a variable, so echoed source can no longer match on any build. Verified the suite still catches a genuine semantic change. |
| 2026-08-30 | Fixed `M-074`: the CI workflow's bare `push:` trigger meant every commit on a PR branch ran the whole matrix twice concurrently, and superseded runs were never cancelled. Because the PTY suites drive real interactive shells against wall-clock deadlines, that self-inflicted contention is what makes them flaky — on commit 8684622 the two concurrent runs disagreed, the canonical suite passing while MSRV failed the same ghost test with nothing recorded in eight seconds. `push` is now scoped to `main` and a `concurrency` group cancels superseded runs. |
| 2026-08-30 | `M-075`: the PTY cases that assert a command was recorded were racing `MBX_HISTORY_TIMEOUT`, the budget MBX is designed to abandon rather than stall the prompt. Raised the tolerant PTY default from 1.0s to 5.0s (production deadline behavior stays asserted in `tests/bash/modules.bash` and in the dedicated 0.10s case), and rebuilt `wait_for_count`'s failure report to name which of the plausible causes actually occurred. Recorded as `Mitigated`, not `Fixed`: the failure did not reproduce in ~22 deliberate attempts across CPU saturation, concurrent PTY binaries, and full-suite runs, so the root cause is not established and a validated fix was not possible. |
| 2026-08-30 | **`M-076`: MBX was a complete no-op on Bash 5.0.** An array `PROMPT_COMMAND` is a Bash 5.1 feature; 5.0 treats the variable as a string and runs element 0 only, so `_mbx_render_prompt` never ran, `PS1` was never set, and the shell kept its stock prompt — while the same assignment silently discarded any pre-existing `PROMPT_COMMAND`, costing users of other frameworks their hook for nothing. Invisible because every assertion inspected the variable rather than its effect, local development is 5.2, and no CI ran 5.0 until this branch added the leg. `_mbx_install_hooks` now installs an array on 5.1+ and a `;`-joined string on 5.0; `tests/bash/smoke.bash` asserts a rendered `PS1` and compares the joined value instead of an element count. Note `smoke.bash` spawns plain `bash` for its inner shells, so testing 5.0 requires a `bash` shim first in `PATH`, not merely running the suite under a 5.0 interpreter. This is what the `HRD-001` Bash matrix was added to find. |
| 2026-08-30 | Fixed `M-077`: `mbx history clear` failed with `database is locked` when another shell held the write lock. `clear` waited out contention on opening its connection but then ran `DELETE FROM history` with no retry, under the 100 ms `BUSY_TIMEOUT_MS` meant for the prompt hot path — where never stalling matters and dropping a record is the designed degradation. That is the wrong policy for a command the user typed and is already waiting on, and two open shells is the ordinary case for a shell integration. `clear` now uses the retry helper this file already had, with a new `USER_COMMAND_BUSY_DEADLINE_MS` (2 s); the hot-path budget is unchanged. Surfaced by a CI failure on PR #53, in a diff that touched no history code. |
| 2026-08-30 | Fixed `M-065`, unblocking `COMP-004` (now `validation`). The completion overlay reserves its rows with `\eD` (IND) *before* `\e7` saves the cursor, so a draw that scrolls the screen can no longer invalidate an absolute save — and caps the draw at `LINES-2` so the reservation itself cannot push the prompt off the top. No DSR (`\e[6n`) round trip was needed: the earlier note that one was required assumed the cursor row had to be *known* when it only had to be made *safe*, and avoiding DSR avoids its timeout and type-ahead risks. `crates/pty/tests/overlay_screen.rs` is no longer `#[ignore]`d and was confirmed to fail against the unfixed code. Review caught a defect the fix itself introduced: capping the draw without capping the selection let cycling and ranked accept address rows that were never drawn, so `_MBX_COMP_OVERLAY_SHOWN` now bounds both (OV-3). Recorded a trap worth remembering: 'is the prompt visible' does not discriminate this bug, because Readline redraws the prompt after a `bind -x` widget returns — the stranded overlay rows are what separate the two. |
| 2026-08-31 | Safety/hardening batch ahead of the M-064 rendering ADR. Bash: C0/DEL gates on ghost history-motion and ranked-completion insert (M-050 recurrence); highlight disarm always clears `ENTER_ARMED` (M-044 recurrence); `${#_MBX_HIGHLIGHT_PLAIN-}` bad substitution on forward-motion (M-078); highlight/ghost CLI wait-or-kill plus exit-status check (M-067 recurrence); `_mbx_comp_identifier_ok` anchored regex (M-056 recurrence); shared `_mbx_jobs_suspend`/`_mbx_wait_or_kill_child`. Rust: create-with-mode store file/dir (M-079); non-blocking history Drop (M-080); bounded exclude glob (M-081); highlight rejects C0 and skips UTF-8 after `\` (M-082); retention `saturating_mul`; socket write timeout. Do not mark `HLT-002`, `COMP-004`, or Phase 6 complete. |
| 2026-08-31 | Accepted ADR 0015: `READLINE_LINE` stays plain; styled bytes paint on one reserved preview row (M-065 IND/DECSC). Implemented in `bash/highlight.bash`. Color is a tty-paint decision because `bind -x` stdout is often a pipe (M-062 fixed). Point units are Unicode scalar counts. Preview-row C0 check must not use an octal glob range that includes ESC (M-083). PTY: `highlight_preview_row_paints_sgr_below_an_intact_prompt`. `HLT-002`/`HLT-003`/Phase 6 `complete`; `HLT-003` p99 stays `deferred`. Overlay COLUMNS-1 clamp: `overlay_clamps_a_wide_row_so_it_does_not_wrap`; `COMP-004` `complete` (type-to-filter GUI `deferred`). `scripts/package-release.bash` dry-run for `REL-001` (tag still maintainer-gated). Stall-until-timeout module cases converted to deadline-relative assertions (M-072 leftover). |
| 2026-08-31 | Fixed `M-084`: `_mbx_tty_clamp_row` split UTF-8 under a C locale (`${#var}` / `${var:i:1}` are bytes in POSIX, scalars in UTF-8). Bash-matrix CI failed `non-ASCII scalars count as two columns` on every 5.x leg; the UTF-8 canonical suite stayed green. Clamp now indexes bytes and consumes a whole UTF-8 sequence per wide scalar. |
