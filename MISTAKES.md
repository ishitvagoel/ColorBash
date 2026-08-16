# MBX mistakes and prevention log

This is a cumulative learning record for work performed by coding agents in this
repository. Every agent must read it before planning or editing. The purpose is
to prevent recurrence, not to assign blame.

## Maintenance contract

- Record only a confirmed mistake backed by code, test output, review evidence, or
  a documented correction. Ordinary backlog, deferred scope, preferences, and
  speculative risks belong in `docs/roadmap.md` or an ADR instead.
- Give every mistake a stable ID, discovery date, and `Open`, `Mitigated`, or
  `Fixed` status.
- Include the failed assumption, observable impact, correction/current state,
  prevention rule, and durable evidence.
- Search by cause before adding an entry. If the same cause recurs, update and
  append evidence to the existing entry rather than duplicating it.
- Never delete a fixed entry. Status and evidence may be updated, but the original
  lesson must remain visible.
- Before handing off authorized edits, check whether a confirmed mistake needs a
  new entry. When a fix lands, update the existing entry's status and evidence in
  the same change. During read-only work, report the needed update instead.
- Do not add empty "no mistakes" entries, blame people or agents, or include
  credentials, private command contents, or sensitive logs.
- During parallel work, one designated writer owns this file.

## M-001 — Bash field codec contradicted MBX1

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: escaping `%`, tab, line feed, and carriage return was enough,
  and decoders only needed the corresponding uppercase forms.
- Impact: other control bytes could produce invalid requests, and lowercase or
  general percent escapes were not decoded consistently with Rust.
- Correction: `bash/protocol.bash` now implements the generic byte codec and the
  cross-language behavior is exercised by module/integration tests.
- Prevention: every protocol codec must share hostile-byte, lowercase-escape,
  control-byte, and cross-language round-trip cases.
- Evidence: `bash/protocol.bash`, `tests/bash/modules.bash`,
  `tests/integration/protocol.bash`, and `docs/protocol.md`.

## M-002 — Response parsing did not prove the exact field count

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: `IFS` plus `read` into a final `extra` variable would detect
  every extra response field.
- Impact: a trailing empty field could be accepted because `extra` was still
  empty, weakening the protocol boundary.
- Correction: response parsing now preserves fields and validates their exact
  count.
- Prevention: test consecutive separators, trailing empty fields, too few fields,
  and too many fields for every response variant.
- Evidence: `bash/protocol.bash` and `tests/bash/modules.bash`.

## M-003 — Rust accepted a percent-decoded NUL

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: validating the encoded input was sufficient before percent
  decoding.
- Impact: `%00` could create a Rust string containing NUL even though MBX1 forbids
  it and Bash variables cannot represent it.
- Correction: the Rust decoder rejects NUL after decoding, and the protocol
  contract documents the rule.
- Prevention: validate invariants after every decoding/transformation boundary and
  test them independently in all language implementations.
- Evidence: `crates/protocol/src/lib.rs`, `docs/protocol.md`, and protocol tests.

## M-004 — Fallback rendering lost safety-critical context

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: the Bash fallback only needed a generally usable prompt,
  rather than semantic parity with the native renderer.
- Impact: production and SSH context disappeared precisely when the helper failed.
- Correction: fallback rendering consumes the same explicit flags/context and
  retains production and SSH warnings. Focused tests cover production precedence
  and SSH without production.
- Prevention: run the same semantic-state matrix against coprocess, per-call, and
  fallback paths. Treat danger context as required behavior, not decoration.
- Evidence: `bash/fallback.bash`, `bash/config.bash`, and
  `tests/bash/modules.bash`.

## M-005 — Render paths independently interpreted policy and mutated `PS1`

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: coprocess, per-call, and fallback adapters could each compute
  flags and update the prompt safely.
- Impact: policy drift made adapters non-substitutable and directly enabled the
  missing-context fallback behavior in `M-004`.
- Correction: one immutable context/flag set is computed per prompt cycle; every
  adapter returns via `REPLY`; only `bash/prompt.bash` commits `PS1`.
- Prevention: adapters for one port must have the same input/output contract and
  must not own coordinator state.
- Evidence: `bash/config.bash`, `bash/engine.bash`, `bash/prompt.bash`, and
  `tests/bash/modules.bash`.

## M-006 — Git execution was weaker than the repository-code security claim

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: invoking bare `git status` was inherently free from
  configured external execution and executable-selection risks.
- Impact: Git could consult configured filesystem-monitor behavior, and an empty
  or relative `PATH` entry could select a repository-local `git`, while the docs
  claimed discovery did not execute repository code.
- Correction: the provider resolves Git once from executable files in absolute
  `PATH` entries, never falls back to bare `git`, and uses fixed arguments that
  disable `core.fsmonitor`, color, and optional locks. Absolute `PATH` entries are
  explicitly treated as trusted caller configuration.
- Prevention: define subprocess specifications explicitly, disable execution
  extension points, and test the constructed command and hostile repository
  configuration before making security claims.
- Evidence: `crates/cli/src/provider.rs`, its hostile `PATH` and
  `core.fsmonitor` tests, and ADR 0007.

## M-007 — Post-collection size checks were described as resource bounds

- Discovered: 2026-08-15
- Status: Mitigated
- Failed assumption: checking `stdout.len()` after `Command::output()`, or checking
  a Bash variable after `read`, limited the bytes acquired from a producer.
- Impact: Git could run indefinitely and allocate more than 1 MiB before
  rejection. A fast coprocess peer could likewise make Bash allocate an
  arbitrarily large line before the 64-KiB decoder check ran; `read -t` limits
  time, not bytes.
- Correction: Git stdout is acquired through a `MAX+1` capped reader with a
  deadline. The normal timeout path attempts direct-child kill/reap and reports a
  typed cleanup failure if it cannot complete. Bash detects raw NUL, reads in
  bounded chunks into at most the framing allowance, and rejects oversized
  terminated or unterminated producers before collecting their complete output.
- Current gap: Git cleanup is intentionally direct-child-only. An unexpected
  descendant holding inherited stdout can outlive the provider, although the
  detached capped reader cannot extend prompt return. Kernel stalls in
  `spawn`/`kill`/`wait` are not independently cancellable.
- Prevention: distinguish acceptance limits from acquisition limits. Require
  capped reads, deadlines, child cleanup, and adversarial oversized/hanging tests.
- Evidence: `crates/cli/src/provider.rs`, `bash/engine.bash`,
  `bash/protocol.bash`, their hanging/oversize/NUL regression tests, ADR 0007,
  and roadmap items `BST-006` and `PRM-003`.

## M-008 — The timeout did not cover the complete fallback chain

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: a deadline on the coprocess read made prompt fallback
  deadline-bounded.
- Impact: after timeout, Bash invokes an unbounded per-call helper and then a
  fallback that can start its own synchronous Git process. Coprocess cleanup also
  performs an unbounded `wait` after sending `TERM`. Any of these steps can still
  block the interactive prompt indefinitely.
- Correction: one absolute deadline now covers request encoding and allocation,
  coprocess exchange, bounded response decode, cleanup, per-call fallback, and
  final process-free Bash fallback. Oversized logical paths, percent-heavy
  fields, stalled helpers, and near-limit responses have focused deadline tests.
- Prevention: assign one deadline budget to the whole operation, including retries
  and fallback, and test it with deliberately hanging helpers/providers.
- Evidence: `bash/engine.bash`, `docs/architecture.md`, and roadmap item
  `PRM-003`.

## M-009 — Direct helper color policy did not account for redirection

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: environment variables alone were enough to choose color for
  every `mbx prompt` caller.
- Impact: direct redirected helper output could contain color despite the UX contract
  that redirected output is plain.
- Correction: prompt defaults now insert `FLAG_NO_COLOR` when stdout is not a
  terminal; explicit `--flags` still replaces defaults so Bash per-call command
  substitution can request color under a pipe (`M-011`).
- Prevention: model caller-supplied capabilities separately from direct-process
  defaults and test terminal versus piped stdout without breaking Bash command
  substitution.
- Evidence: `crates/cli/src/environment.rs`, `tests/integration/protocol.bash`,
  `crates/cli/src/cli.rs`, `tests/bash/modules.bash`, and
  `docs/prm-002-redirected-output-plan.md`.

## M-010 — Shared startup captured prompt-only current-directory state eagerly

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: all commands could share one eager environment snapshot.
- Impact: help, version, handshake, and socket commands could fail when invoked
  from a deleted or otherwise unavailable current directory, even though only
  prompt rendering needed it.
- Correction: CLI parsing resolves prompt defaults lazily only for the `prompt`
  command.
- Prevention: capture side-effectful or fallible defaults at the narrowest use-case
  boundary and add tests proving unrelated commands do not invoke them.
- Evidence: `crates/cli/src/cli.rs`, `crates/cli/src/environment.rs`, and
  `tests/integration/protocol.bash`.

## M-011 — Child stdout was mistaken for the caller's terminal capability

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: the helper's `stdout.is_terminal()` was a universal color
  default.
- Impact: Bash per-call mode captures helper output through command substitution,
  so the child always saw a pipe and silently lost requested color.
- Correction: Bash owns and passes its terminal capability flags; Rust no longer
  overrides them from command-substitution stdout. A focused test exercises the
  actual color-enabled per-call command-substitution topology.
- Current follow-up: redirected-output defaults are fixed (`M-009`); width model
  work remains `PRM-002` discovery.
- Prevention: distinguish transport characteristics from end-user display
  capabilities and test each adapter under its real process topology.
- Evidence: `bash/config.bash`, `bash/engine.bash`,
  `crates/cli/src/environment.rs`, and `tests/bash/modules.bash`.

## M-012 — Crate-internal extension seams were not initially constructible

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: marking traits and outer types `pub` inside private modules
  was sufficient for crate-internal substitutability.
- Impact: another module could not construct `ProviderError` or a custom `Theme`,
  so advertised provider/theme extension seams were not actually usable.
- Correction: provider errors expose a constructor and theme fields are
  constructible by sibling modules inside the crate. A sibling-module compile
  test constructs both types through the intended crate-internal boundary.
- Prevention: compile substitutes from the intended crate-internal consumer
  module; review constructors, associated errors, DTO fields, and lifetimes, and
  do not describe these private-module seams as an external public API.
- Evidence: `crates/cli/src/provider.rs`, `crates/cli/src/prompt.rs`, and
  `seam_contract_tests` in `crates/cli/src/lib.rs`.

## M-013 — The first canonical roadmap draft contained circular gates

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: a phase could both be blocked by a gate and contain the work
  required to produce that gate, and early research could depend on later editor
  experiments without deadlocking phase completion.
- Impact: Phase 0 and completion could never complete as written. Phase 3A and
  full Phase 3 also had an ambiguous exit, and several absolute history guarantees
  were not technically measurable.
- Correction: gate-producing completion experiments are separate from popup work;
  Phase 0 exits at `G0`; `G2` is explicitly a Phase 3A gate; later phase exits have
  stable deliverables; history guarantees use controlled comparisons and bounded
  budgets.
- Prevention: for every gate, identify exactly one producer set and its downstream
  consumers, then check the dependency graph for cycles. Express interactive and
  stateful guarantees as measurable bounds or controlled comparisons rather than
  unexplained `never` claims.
- Evidence: `docs/roadmap.md` gate map, `G0`-`G5`, phase tables, and change log.

## M-014 — An ambiguous patch changed the wrong repeated status lines

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: multiple identical `- Status: Fixed` lines could be patched
  safely without including each entry heading as context.
- Impact: the patch temporarily changed the statuses of `M-001`, `M-005`, and
  `M-010` instead of all intended entries.
- Correction: every affected status was checked against its heading and restored
  to its then-intended value; all later status changes are likewise anchored to
  the stable entry heading.
- Prevention: anchor edits to repeated Markdown fields with the stable entry ID or
  heading, then immediately verify the complete ID-to-status mapping.
- Evidence: the current `MISTAKES.md` ID/status map and the post-edit schema check.

## M-015 — The framing limit depended on the line-ending convention

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: reading at most `MAX_MESSAGE_BYTES + 2` bytes and checking
  the count before trimming covered EOF, LF, and CRLF uniformly.
- Impact: the Rust transport accepts an exactly 64-KiB protocol line followed by
  LF but rejects the same line followed by CRLF. Equivalent peers therefore get
  different behavior at the documented boundary.
- Correction: both clients normalize EOF/LF/CRLF framing before applying the
  65,536-byte payload limit, and Rust/Bash tests cover `MAX-1`, `MAX`, and
  `MAX+1` under every terminator.
- Prevention: define whether delimiters are inside the payload limit, normalize
  them before applying that limit, and test `MAX-1`, `MAX`, and `MAX+1` with EOF,
  LF, and CRLF.
- Evidence: `crates/cli/src/transport.rs`, `docs/protocol.md`, and roadmap item
  `BST-006`.

## M-016 — Bash fallback sanitization covered only selected controls

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: replacing escape, tab, line feed, carriage return, and Bash
  expansion characters was equivalent to rejecting the full control range.
- Impact: other C0 bytes and DEL can reach fallback `PS1`, so the native and
  fallback renderers do not satisfy the same terminal-safety contract.
- Correction: the fallback replaces every C0 byte, DEL, `$`, backticks, and
  backslashes, and native/per-call/fallback paths share a hostile-input corpus.
- Prevention: define one renderer safety postcondition for every adapter and run
  the same C0, DEL, expansion-character, length, and hostile-context corpus against
  native and fallback output.
- Evidence: `bash/fallback.bash`, `crates/cli/src/prompt.rs`, and roadmap item
  `PRM-007`.

## M-017 — Per-call prompt serialization discarded unknown flag bits

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: translating the currently known flag bits into named CLI
  switches was equivalent to forwarding the additive `PromptFlags` value.
- Impact: the coprocess path preserves unknown bits, while per-call mode silently
  drops them. A newly added capability can therefore change meaning when the
  transport falls back, violating the adapter-substitution contract.
- Correction: `mbx prompt --flags <u32>` accepts the complete additive value;
  Bash per-call mode forwards it directly, and later named switches mutate known
  bits without discarding unknown ones. All adapters have parity tests.
- Prevention: preserve additive values through every transport representation and
  test an unknown bit across coprocess, per-call, and fallback adapters before
  adding a new flag.
- Evidence: `bash/config.bash`, `bash/engine.bash`, `docs/protocol.md`, and roadmap
  item `PRM-008`.

## M-018 — Provider absence and cache outcomes were initially conflated

- Discovered: 2026-08-15
- Status: Mitigated
- Failed assumption: every nonzero Git exit could be represented as repository
  absence, and successful-cache tests were enough to define cache behavior.
- Impact: a status failure after repository discovery could disappear without a
  typed diagnostic, while negative and transient-error caching had no explicit,
  regression-tested policy.
- Correction: a fixed worktree preflight distinguishes ordinary absence from a
  later typed `CommandFailure`. `Some`, `None`, and `Err` results are deliberately
  cached for the bounded one-second TTL, with deterministic expiry, invalidation,
  and capacity tests.
- Current gap: a rare fatal preflight failure remains indistinguishable from a
  non-repository because the provider deliberately does not acquire or expose Git
  stderr.
- Prevention: specify presence, absence, and failure independently, then define
  cacheability, TTL, diagnostics, and refresh behavior for each outcome.
- Evidence: `crates/cli/src/provider.rs`, its provider/cache tests, and ADR 0007.

## M-019 — PTY wait predicates could over-read past the next prompt

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: waiting for an output marker with a PTY read loop returned
  exactly up to that marker, so a following wait for the next prompt would
  observe it.
- Impact: a single read chunk can contain the matched output plus the trailing
  prompt, so the subsequent wait for the prompt timed out or matched stale
  echo; history dump reads through PTY echo raced with prompt timing and could
  return partial or empty content. The pattern later recurred in the Phase 3A
  recording tests: they sent `exit` after matching command output but before
  proving the following prompt/history exchange had completed, so parallel runs
  lost the final expected row during helper teardown. It recurred again in the
  HIST-007 write-ack W-5 harness, which ran `true` and waited only for `> `, so
  a leftover prompt could satisfy the wait before ACK samples were written
  (first release run collected 0 samples).
- Correction: waits that must observe a full output-plus-prompt sequence use one
  predicate requiring every needle in one read (`wait_all`). History content is
  read from the `HISTFILE` on disk after a sourced dump script prints a marker
  that never appears in typed-command echo, so assertions never depend on
  readline echo or prompt timing. History-recording tests likewise wait for
  output plus the next prompt, then poll for the asynchronous commit while the
  helper remains alive before exiting the shell. Write-ack W-5 now types
  `echo bench-{n}` and `wait_all`s for the echoed marker plus `> `.
- Prevention: when a test needs both a command's output and the following
  prompt, wait for both in a single read; never re-wait after a match that may
  have consumed the trailing prompt. Synchronize on asynchronous artifacts
  before terminating their producer, and read file artifacts from disk when the
  assertion is about file contents.
- Evidence: `crates/pty/tests/history_admission.rs`,
  `crates/pty/tests/history_recording.rs`,
  `crates/pty/tests/multiline_width.rs`,
  `crates/pty/tests/history_write_ack.rs`, and
  `docs/research/bash-history-admission.md`.

## M-020 — History-off `HISTCMD` behavior was asserted without evidence

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: commands typed while history was disabled would still
  advance `HISTCMD`, and the research note recorded that behavior without a
  focused PTY assertion.
- Impact: the history capture contract could use an incorrect sequence model and
  treat omitted commands as admitted events.
- Correction: the PTY characterization now asserts that `HISTCMD` does not
  advance during history-off commands, that `set -o history` itself is omitted
  when read while history is disabled, and the research/ADR evidence describes
  the admitted-entry-only counter accurately.
- Prevention: every Bash capture-semantic claim must have a controlled PTY test;
  do not promote an unverified shell assumption into an evidence document.
- Evidence: `crates/pty/tests/history_admission.rs`,
  `docs/research/bash-history-admission.md`, and ADR 0005.

## M-021 — Near-limit Bash transport timing was under-budgeted in a regression test

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: a maximum-size, printable-ASCII MBX1 request would always
  reach a Bash fixture within a 30-ms render budget, independent of host load
  and Bash pipe-copy speed.
- Impact: the canonical module suite reported that a fitting request was not
  sent even though the protocol encoder accepted the exact 64-KiB frame; the
  test produced a false negative for bounded transport behavior.
- Correction: the near-limit stalled-helper fixture now uses an explicit 100-ms
  render budget and retains a 200-ms end-to-end bound while still proving that
  no second per-call budget is granted.
- Prevention: distinguish protocol acceptance limits from transport acquisition
  budgets and benchmark max-size Bash writes before choosing a regression-test
  deadline.
- Evidence: `tests/bash/modules.bash`, `bash/engine.bash`, and the passing
  canonical suite.

## M-022 — `fc -ln -1` in a command substitution lags the newest history entry

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: `fc -ln -1` inside `$(...)` returns the newest Bash-admitted
  history entry at the prompt boundary, matching `history 1`.
- Impact: the history recorder observed a stale entry (the previous command) at
  every prompt, producing duplicate and misattributed records; the research
  doc's admission-authority evidence could not be honored.
- Correction: the recorder reads `history 1` instead and strips the
  right-aligned number plus two-space separator, preserving a user-typed
  leading space; Bash 5.2.21 `fc` in a command substitution lags by one entry.
- Prevention: validate admission-reading primitives against `history 1` under
  a genuine PTY before wiring capture, and assert stored text against the
  folded HISTFILE form.
- Evidence: `bash/history.bash`, `crates/pty/tests/history_recording.rs`, and
  `docs/research/bash-history-admission.md`.

## M-023 — Temporary history debugging leaked command text and broke compilation

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: ad hoc `MBX_DBG` file loggers were safe to leave across the
  Bash recorder, Rust ingestion/writer, helper startup, and PTY scaffolding while
  diagnosing tests.
- Impact: both debug branches treated `var_os` as a `Result` instead of an
  `Option`, preventing the crate from compiling. The writer branch also copied
  sensitive command text into an unmanaged debug file outside the protected
  SQLite store. Bash-side cycle/exclusion/exchange diagnostics likewise copied
  full history entries, while PTY scaffolding enabled and dumped those files,
  contradicting the accepted no-command-text logging contract.
- Correction: the Rust and Bash debug channels, helper redirection, and PTY dump
  scaffolding were removed. Storage errors continue through the standard trace
  path, whose diagnostic contains only the typed failure kind.
- Prevention: history diagnostics must use the standard command-text-free trace
  boundary. Add a focused assertion with sentinel secret text whenever a new
  diagnostic is introduced; never add local debug output for a history record.
  The Bash module contract rejects any reintroduced `MBX_DBG` channel.
- Evidence: `bash/history.bash`, `bash/engine.bash`,
  `crates/cli/src/storage.rs`,
  `storage_failure_diagnostic_exposes_only_the_typed_kind`,
  `crates/pty/tests/history_recording.rs`, `tests/bash/modules.bash`, and the
  passing canonical suite.

## M-024 — Unset history configuration enabled the opt-in sidecar

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: treating only `MBX_HISTORY=0` as disabled was equivalent to
  making enhanced history opt-in.
- Impact: an unset variable enabled Bash capture and made every helper server
  open or create the history store without explicit consent. This contradicted
  the accepted privacy contract and could also make unrelated helper handshakes
  fail when the default data directory was unavailable.
- Correction: Bash and Rust now enable history only for the exact value
  `MBX_HISTORY=1`. History CLI tests opt in explicitly, and a PTY regression test
  proves that the unset default creates no store.
- Prevention: every opt-in feature must test absent, explicit-off, and
  explicit-on configuration. Composition roots must not allocate an opt-in
  resource until the positive enablement value has been established.
- Evidence: `bash/history.bash`, `crates/cli/src/policy.rs`,
  `crates/pty/tests/history_recording.rs`, and the focused Bash module suite.

## M-025 — Newer Clippy rejected source and test idioms

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: Rust code accepted by the minimum Rust toolchain would
  remain warning-free under the newer stable Clippy used by CI.
- Impact: CI's warnings-as-errors gate rejected a Linux no-op `u32` cast, a
  manual slice membership scan, an `expect` after an `is_some` check, and a
  cloned value used only to build a one-element comparison slice. The otherwise
  passing workspace could not compile under Rust 1.97.
- Correction: the termios assertion now uses target-specific `ISIG` types for
  Linux and macOS without a cast, the history assertion uses slice `contains()`
  directly, completed provider output is assembled through `Option`
  combinators, and the executable probe assertion uses `slice::from_ref`.
- Prevention: keep CI on stable Clippy, fix new lints without blanket allows,
  and model platform-dependent FFI widths explicitly rather than casting tests
  to the current host type.
- Evidence: `crates/pty/tests/driver.rs`,
  `crates/pty/tests/history_admission.rs`, `crates/cli/src/provider.rs`, and the
  warnings-denied Clippy gate.

## M-026 — Greedy `history 1` stripping dropped a leading space

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: removing the longest prefix ending in two spaces from
  `history 1` was equivalent to stripping the right-aligned number and its
  two-space separator.
- Impact: an admitted command that itself began with a space lost that space in
  the sidecar, so the stored text was not the folded HISTFILE form.
- Correction: the recorder parses `^[[:space:]]*[0-9]+  (.*)$` and keeps the
  remainder, including a user-typed leading space.
- Prevention: assert stored sidecar text against the exact folded HISTFILE form,
  including leading spaces, rather than only substring presence.
- Evidence: `bash/history.bash` and
  `leading_space_is_preserved_when_admitted` in
  `crates/pty/tests/history_invariance.rs`.

## M-027 — `HISTCMD` was used as the non-admission drop key

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: if `history 1` still showed the previous entry, comparing
  `HISTCMD` would detect that Bash had not admitted a new command.
- Impact: while history was off, the recorder re-queued the same `set +o history`
  row and never recorded the later admitted command, so omissions and later
  admissions were both wrong.
- Correction: the drop key is the list number printed by `history 1`, which is
  the identity of the newest admitted entry. That same list number is the stored
  diagnostic `history_number`. `HISTCMD` is not used for drop detection or
  storage.
- Prevention: treat `HISTCMD` as unstable; do not store it as `history_number`.
  PTY tests must cover history-off followed by a later admitted command, not
  only that omitted text is absent.
- Evidence: `bash/history.bash` and `history_off_commands_are_not_recorded` in
  `crates/pty/tests/history_invariance.rs`.

## M-028 — The first prompt recorded a seeded `HISTFILE` entry

- Discovered: 2026-08-15
- Status: Fixed
- Failed assumption: every prompt-boundary `history 1` read was a newly completed
  command in this session.
- Impact: loading a prior `.bash_history` made the sidecar record that old entry
  before the user typed anything, violating the post-completion capture contract.
- Correction: the recorder skips recording on the first prompt but still snapshots
  the `history 1` list number and command text as the drop key. A seeded-file PTY
  test asserts count 0 at the first prompt and after an empty Enter.
- Prevention: session-start prompts are not completion boundaries; snapshot the
  drop key without recording and test seeded `HISTFILE` plus empty Enter.
- Evidence: `bash/history.bash` and `seeded_histfile_is_not_rewritten_on_append`
  in `crates/pty/tests/history_invariance.rs`.

## M-029 — Row-cap prune ran a full ordered delete under the limit

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: `DELETE ... LIMIT -1 OFFSET max_rows` would be cheap when
  the store had fewer rows than the retention cap.
- Impact: the writer pruned every 32 inserts with a full `ORDER BY` delete, so a
  100k-row corpus load did not finish in minutes and could not produce `G2`
  query evidence.
- Correction: count first and skip the ordered delete when `count <= max_rows`.
- Prevention: retention maintenance must be cheap on the common under-cap path;
  a 100k load (or a batch larger than `WRITER_BATCH_SIZE`) is required evidence,
  not only a handful of rows.
- Evidence: `prune` in `crates/cli/src/storage.rs` and
  `row_cap_prune_keeps_every_row_under_the_limit`.

## M-030 — Writer autocommit made large ingest unbounded

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: WAL autocommit per insert was fast enough that batching
  could wait; `HIST-012` "batches where practical" was treated as optional.
- Impact: a 100k-row corpus fill saturated the queue and waited on per-row
  commits, so query-percentile evidence still could not be collected after
  `M-029`.
- Correction: the writer opens `BEGIN IMMEDIATE` for each batch of
  `WRITER_BATCH_SIZE` inserts and commits before prune; shutdown commits a
  partial batch.
- Prevention: ingest of the `HIST-004` 100k corpus is required writer evidence,
  not only a handful of rows. Queue acknowledgement remains "accepted by the
  queue", not "committed".
- Evidence: `writer_loop` in `crates/cli/src/storage.rs` and the ignored
  `load_100k_and_measure_query_percentiles` test.

## M-031 — Failed batch commit left the writer in an open transaction

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: tracing a failed `COMMIT` and resetting `pending` was enough;
  SQLite could still be inside the batch transaction.
- Impact: the next `BEGIN IMMEDIATE` failed, every later record in that session
  was dropped, and the uncommitted batch was lost on shutdown because `pending`
  had already been cleared.
- Correction: on batch or shutdown `COMMIT` failure, `ROLLBACK` before clearing
  `pending`; prune only after a successful batch commit.
- Prevention: every writer transaction path must pair failed commits with
  rollback; do not prune inside a failed batch.
- Evidence: `writer_loop` in `crates/cli/src/storage.rs`.

## M-032 — Concurrent store open lost migration and read paths on writer locks

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: a single `busy_timeout` on each statement was enough for
  eight simultaneous first opens and for search/count paths that reused write
  connections with `PRAGMA journal_mode=WAL`.
- Impact: concurrent-writer contention tests failed with `database is locked`
  during migrate or reader `open`; one row could be dropped when
  `BEGIN IMMEDIATE` failed once under cross-session WAL contention.
- Correction: migrate inside `BEGIN IMMEDIATE` with version re-check and a
  bounded retry loop; skip redundant WAL mode changes; open read-only
  connections for search/count; retry writer `BEGIN IMMEDIATE`/`COMMIT` on
  `SQLITE_BUSY`/`SQLITE_LOCKED` within `BUSY_TIMEOUT_MS`.
- Prevention: any path that may run concurrently on one sidecar file must use
  read-only handles for queries, transactional idempotent migration, and writer
  lock retries before dropping queued entries.
- Evidence: `open_read_connection`, `try_migrate`, and `execute_batch_with_lock_retry`
  in `crates/cli/src/storage.rs`; concurrent-writer tests C-1–C-3 and C-6.

## M-033 — Unconditional chmod widened restrictive store modes

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: `set_permissions(0700/0600)` on every open was equivalent to
  ADR 0005's "never make existing files more permissive" rule and covered WAL/SHM
  mode bits.
- Impact: a store left at owner-read-only `0400` or `0000` was widened to `0600`
  on reopen; `-wal`/`-shm` could remain at umask defaults because only the main
  database path was chmod'd.
- Correction: newly created directories and databases receive `0700`/`0600`;
  existing paths use `tighten_mode` (`(current & 0o777) & max_mode`) so bits are
  never added. `restrict_store_permissions` then tightens the directory,
  database, `-wal`, and `-shm` after WAL setup and migration.
- Prevention: permission tests must cover WAL/SHM sidecars, world-accessible
  tightening, and restrictive modes that must not widen. Assign target modes
  only on first create; never assign them unconditionally on every open.
- Evidence: `tighten_mode`, `restrict_store_permissions`, and permission tests
  P-1–P-4 in `crates/cli/src/storage.rs`; `docs/history-g2-permission-plan.md`.

## M-034 — Open writer batches were invisible to live readers

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: queue acknowledgement plus eventual shutdown commit was
  enough for PTY and CLI readers to observe rows while the helper stayed alive.
- Impact: with `WRITER_BATCH_SIZE=32`, partial batches stayed inside an open
  `BEGIN IMMEDIATE` transaction until 32 inserts or helper shutdown. External
  `mbx history count` and PTY `wait_for_count` saw `count=0` even after ACK,
  breaking invariance evidence despite a populated queue.
- Correction: when the writer queue is idle and `pending > 0`, commit the
  partial batch without prune; busy queues still batch to 32 (`M-030`).
- Prevention: any path that lets live readers query while the writer is alive
  needs storage tests that keep the writer open (V-1–V-2) and PTY invariance
  (V-3). Do not weaken `wait_for_count` or change ACK meaning to hide the gap.
- Evidence: `writer_loop`, V-1–V-2 in `crates/cli/src/storage.rs`, V-3 in
  `crates/pty/tests/history_invariance.rs`, and `docs/history-g2-idle-commit-plan.md`.

## M-035 — Concurrent store open and writer begin still dropped rows under WAL load

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: migrate's 2 s retry loop and the writer's 100 ms
  `BEGIN IMMEDIATE` retry were enough for eight simultaneous first opens and
  cross-session WAL writers.
- Impact: GitHub Actions and local stress runs intermittently failed
  `concurrent_distinct_sessions_both_land` with `database is locked` on open and
  `concurrent_sessions_write_distinct_rows_without_duplicates` with `255` rows
  instead of `256` when a writer dropped a queued entry after a short lock wait.
- Correction: retry the full `open_connection` path on lock contention for up to
  `MIGRATE_BUSY_DEADLINE_MS`, and use that same deadline for writer
  `BEGIN IMMEDIATE` instead of the 100 ms statement budget.
- Prevention: concurrent-writer storage tests must be stress-run under parallel
  `cargo test --test-threads=8`; any writer path that drops after lock contention
  needs a bounded retry before acceptable loss.
- Evidence: `open_connection`, `execute_batch_with_lock_retry_until`, and
  `writer_loop` in `crates/cli/src/storage.rs`; CI run failure on
  https://github.com/ishitvagoel/ColorBash/actions/runs/31933197095.

## M-036 — Socket transport test wrote before the client request landed

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: a stub server could `writeln!` a mismatched MBX1 line
  immediately after `accept()` and still exercise response-id rejection on the
  real socket client path.
- Impact: on GitHub Actions the server thread sometimes closed the socket before
  `SocketClient::exchange` wrote the request, so CI saw `Broken pipe (os error
  32)` instead of `response id 10 does not match request id 9` and
  `tests/run.bash` exited 101 on commit `a80172b`.
- Correction: socket integration tests now read the client request with
  `read_bounded_line` before writing the mismatched response; the handshake
  regression asserts the server observed `MBX1\t9\tPING` before responding.
- Prevention: socket transport tests must not send a response until the client
  request line is read; keep the deterministic `ClientSession`/`Cursor` unit
  test and add a request-handshake assertion when exercising real Unix sockets.
- Evidence: `socket_client_rejects_a_mismatched_response_id` and
  `socket_client_rejects_a_mismatched_response_id_after_request_handshake` in
  `crates/cli/src/transport.rs`; CI run failure on
  https://github.com/ishitvagoel/ColorBash/actions/runs/31934877398.

## M-037 — Completion test fixtures installed in every interactive session

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: obscure `mbx_comp_*` names were safe to define and bind
  from `_mbx_completion_install` because only tests would invoke them.
- Impact: every interactive MBX session defined `mbx_comp_probe`,
  `mbx_comp_flag`, and `mbx_comp_flag_nospace`, and installed `complete -F`
  wrappers for those names. Invoking a flag fixture printed `GOT:` output.
- Correction: fixtures install only when `MBX_COMP_FIXTURES=1`. Default
  `_mbx_completion_install` and `bash/init.bash` define none of those names.
- Prevention: every opt-in test seam must test absent, explicit-off, and
  explicit-on configuration. Composition roots must not define test commands
  until the positive enablement value has been established.
- Evidence: `bash/completion.bash`, `default_install_does_not_define_fixtures`
  in `crates/pty/tests/completion_harness.rs`, and `tests/bash/modules.bash`.

## M-038 — Ranked-accept PTY assert accepted Tab-only insertion

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: a PTY output substring of `aaflag` proved the ranked-accept
  chord spliced `_MBX_COMP_RANKED_REPLY`.
- Impact: unique Tab completion of `aaflag` would also contain that substring, so
  the test could pass if the chord were a no-op. Host bytes were
  `\nGOT:aaaaflag|` (`aa` + spliced `aaflag`).
- Correction: A-1 and A-5 wait for the exact accepted line. After M-039 that
  line is `GOT:aaflag|` (current-word replace). A-3 types `echo ok` before the
  chord so a no-snapshot insert cannot hide in a later command. Tab without the
  chord keeps `GOT:aa|`.
- Prevention: completion-plus-insert PTY tests must assert the exact line bytes
  that distinguish ranked accept from stock Tab insertion.
- Evidence: `ranked_accept_inserts_top_ranked_bytes`,
  `ranked_accept_tab_without_chord_keeps_prefix`, and
  `ranked_accept_metadata_never_inserted` in
  `crates/pty/tests/completion_harness.rs`; `docs/comp-004-ranked-accept-plan.md`.

## M-039 — Ranked-accept spliced at the cursor instead of replacing the word

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: editor-style splice at `READLINE_POINT` was the right
  action for a ranked completion accept.
- Impact: prefix `aa` plus ranked `aaflag` became `aaaaflag`. A stale ranked
  snapshot could also splice into a later unrelated word on the same line.
- Correction: `_mbx_comp_accept_ranked` replaces the current whitespace-delimited
  word when it is a non-empty prefix of `_MBX_COMP_RANKED_REPLY`. Unrelated words
  are left unchanged. The snapshot is cleared at the next prompt.
- Prevention: a completion-accept action must replace the current word, not
  splice after it. PTY tests must cover Tab-without-chord, accept-with-prefix,
  and a stale unrelated word.
- Evidence: `_mbx_comp_accept_ranked` in `bash/completion.bash`;
  `ranked_accept_inserts_top_ranked_bytes`,
  `ranked_accept_tab_without_chord_keeps_prefix`, and
  `ranked_accept_refuses_stale_unrelated_word` in
  `crates/pty/tests/completion_harness.rs`.

## M-040 — Default search chord collided with stock Readline

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: `\C-x\C-r` was a free emacs chord analogous to ranked-accept
  `\C-x\C-a`, so occupied-skip would only fire for user bindings.
- Impact: stock emacs binds `\C-x\C-r` to `re-read-init-file`. The search
  installer skipped the chord, `_MBX_SEARCH_BOUND` stayed `0`, and PTY insert
  cases submitted the typed prefix instead of a sidecar match.
- Correction: default chord is `\C-xh` (Ctrl-X then `h`). `\C-x\C-r` is
  occupied; `\C-x\C-s` is terminal XOFF under IXON and freezes PTY output.
  S-7 asserts `_MBX_SEARCH_BOUND=1` on a default install.
- Prevention: before choosing a default `bind -x` chord, inspect `bind -p` on
  stock emacs and vi-insert, and avoid `C-s`/`C-q` flow-control bytes. Occupied-
  skip tests that pre-bind a fake occupant do not prove the default chord is
  free or PTY-safe. Add a default-install bound assertion and an insert PTY case.
- Evidence: `bash/search.bash`, `default_chord_installs_on_stock_emacs` in
  `crates/pty/tests/history_search.rs`, and ADR 0009.

## M-041 — Protocol frame reader rejected multi-line search output

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: `_mbx_read_bounded_response` could collect sidecar search
  lines the same way it collects one MBX1/MBX2 frame.
- Impact: a helper that printed two command lines delivered both LFs in one
  `read`, so the protocol reader rejected the buffer (`before_lf` still
  contained a newline). Bounded cycling never left the first snapshot.
- Correction: search uses a one-line `read -r` helper that stops at the first
  LF and leaves later lines in the pipe.
- Prevention: protocol frame readers are single-payload. CLI output that is
  one record per line needs a line reader plus a focused two-line contract
  test. Do not reuse `_mbx_read_bounded_response` outside framing.
- Evidence: `_mbx_search_read_line` in `bash/search.bash` and the two-line
  cycle contract in `tests/bash/modules.bash`.

