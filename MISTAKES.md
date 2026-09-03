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
  (first release run collected 0 samples). Recurrence (2026-08-27, GitHub
  Actions): `next_prompt_usable_after_insert_and_ctrl_c` waited only for `^C`
  then typed `printf`; under CI load the next prompt was not ready, so the
  first byte was lost (`rintf`) and the follow-up never printed
  `MBX_EDT:after_cancel`. A later run waited for `> ` after `^C` but sent
  Ctrl+C before the bind -x insert finished, so the capture stayed `^C` with
  no prompt.
- Correction: waits that must observe a full output-plus-prompt sequence use one
  predicate requiring every needle in one read (`wait_all`). History content is
  read from the `HISTFILE` on disk after a sourced dump script prints a marker
  that never appears in typed-command echo, so assertions never depend on
  readline echo or prompt timing. History-recording tests likewise wait for
  output plus the next prompt, then poll for the asynchronous commit while the
  helper remains alive before exiting the shell. Write-ack W-5 now types
  `echo bench-{n}` and `wait_all`s for the echoed marker plus `> `. The editor
  Ctrl+C follow-up waits until `> ` appears after `^C` in the same capture
  before typing.
- Prevention: when a test needs both a command's output and the following
  prompt, wait for both in a single read; never re-wait after a match that may
  have consumed the trailing prompt. Synchronize on asynchronous artifacts
  before terminating their producer, and read file artifacts from disk when the
  assertion is about file contents.
- Evidence: `crates/pty/tests/history_admission.rs`,
  `crates/pty/tests/history_recording.rs`,
  `crates/pty/tests/multiline_width.rs`,
  `crates/pty/tests/history_write_ack.rs`,
  `crates/pty/tests/editor_bind_x.rs`, and
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
  Recurrence (2026-08-27, Clippy 1.98 `manual_contains`): highlight keyword
  membership used `KEYWORDS.iter().any(|keyword| *keyword == word)` and failed
  CI `-D warnings`; it now uses `KEYWORDS.contains(&word)`.
- Prevention: keep CI on stable Clippy, fix new lints without blanket allows,
  and model platform-dependent FFI widths explicitly rather than casting tests
  to the current host type.
- Evidence: `crates/pty/tests/driver.rs`,
  `crates/pty/tests/history_admission.rs`, `crates/cli/src/provider.rs`,
  `keyword_kind` in `crates/cli/src/highlight.rs`, and the warnings-denied
  Clippy gate.

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
  `wait_for_count` requires exact equality: if two admitted commands can land
  in one idle batch, wait for the true total (not an intermediate count that
  the poll can skip). Recurrence (2026-08-27, GitHub Actions):
  `insert_restore_signal_and_resize_preserve_stty` ran `STTY1` then `alpha`
  and waited for `1`; CI committed both rows at once (`last=2`). A later run
  waited for `> ` after Ctrl+C and typed STTY2 onto the unrestored search
  line (`rintf`). That case now waits for restore, then `^C` plus a new
  prompt, before the follow-up.
- Evidence: `writer_loop`, V-1–V-2 in `crates/cli/src/storage.rs`, V-3 in
  `crates/pty/tests/history_invariance.rs`,
  `insert_restore_signal_and_resize_preserve_stty` in
  `crates/pty/tests/history_search.rs`, and
  `docs/history-g2-idle-commit-plan.md`.

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
  word when it is a non-empty prefix of `_MBX_COMP_RANKED_REPLY` **and** the
  word still starts at the Tab snapshot offset. Unrelated words and later
  prefix-colliding words (`echo aa` after Tab on `aa`) are left unchanged.
  The snapshot is cleared at the next prompt.
- Prevention: a completion-accept action must replace the current word, not
  splice after it. PTY tests must cover Tab-without-chord, accept-with-prefix,
  and a stale unrelated word. Module tests must also cover a prefix-colliding
  word at a different offset. Ranked-cycle must also replace when the current
  word equals `_MBX_COMP_RANKED_REPLY` (prefix-only cannot rotate `aaflag` to
  `zzflag`) and must not rotate the list unless replacement is allowed.
- Evidence: `_mbx_comp_accept_ranked` / `_mbx_comp_ranked_word_eligible` in
  `bash/completion.bash`;
  `ranked_accept_inserts_top_ranked_bytes`,
  `ranked_accept_tab_without_chord_keeps_prefix`, and
  `ranked_accept_refuses_stale_unrelated_word` in
  `crates/pty/tests/completion_harness.rs`. Ranked-cycle: `_mbx_comp_cycle_ranked`,
  `ranked_cycle_next_rotates_from_accepted_head`, and
  `ranked_cycle_refuses_stale_unrelated_word`.

## M-040 — Default ghost strip chord collided with stock Readline

- Discovered: 2026-08-17
- Status: Fixed
- Failed assumption: `\C-xg` was a free emacs chord because it is not a
  `Ctrl-X Ctrl-` control pair.
- Impact: stock emacs binds `\C-xg` to `glob-list-expansions`. Ghost install
  treated the strip chord as occupied, skipped every self-insert wrap, and
  `_MBX_GHOST_BOUND` stayed `0`. Typing worked through stock `self-insert`
  with no suffix.
- Correction: ghost no longer uses a bind -x strip chord. Helper chords are
  inspected on stock emacs (`bind -p`): default kill-line `\C-x\C-k` and
  accept-line `\C-x\C-m`. G-5 asserts `_MBX_GHOST_BOUND=1` on a default
  ghost+history install. A letter suffix such as `\C-xg` / `\C-xj` is not
  used because those collide with stock functions or wrapped `self-insert`.
- Prevention: before choosing a default chord, inspect `bind -p` on stock
  emacs. Occupied-skip that aborts the whole installer must have a
  default-install bound assertion. Avoid `C-s`/`C-q` flow-control bytes.
- Evidence: `bash/ghost.bash`, `default_install_sets_bound_flag` in
  `crates/pty/tests/ghost.rs` (`_MBX_GHOST_BOUND`, `_MBX_GHOST_CYCLE_BOUND`, and
  `_MBX_GHOST_VI_BOUND`), and ADR 0010. Recurrence: `\C-x\C-r` was occupied by
  stock `re-read-init-file`, so history search defaults to `\C-xh` with
  `default_chord_installs_on_stock_emacs` in `crates/pty/tests/history_search.rs`
  and ADR 0009. `\C-x\C-s` is terminal XOFF under IXON. Restore defaults to
  `\C-xl` with `default_restore_chord_installs_on_stock_emacs`.

## M-041 — bind -x inside a keyseq macro drops remaining keys

- Discovered: 2026-08-17
- Status: Fixed
- Failed assumption: `"\C-m": "\C-x\C-n\C-j"` could run a bind -x strip
  function and then stock `accept-line`.
- Impact: Readline discards keys after a bind -x step. Enter either kept the
  unaccepted suffix (G-1 executed the full suggestion) or required `eval` of
  `READLINE_LINE`, which is out of scope and skips `accept-line` / sidecar
  admission / prompt hooks.
- Correction: while a suffix is active, `\C-m` and `\C-j` (newline / `icrnl`
  Enter) are a Readline-only macro: reserved `kill-line` from point, then
  reserved `accept-line`. The line is not evaluated from bind -x. G-1 asserts
  sidecar admission of the typed prefix.
- Prevention: never chain bind -x with later macro keys. Enter and other
  accept paths must remain Readline functions unless an ADR records a
  different execution owner.
- Evidence: `_mbx_ghost_arm_enter` in `bash/ghost.bash`;
  `typing_shows_suffix_and_enter_runs_typed_prefix` in
  `crates/pty/tests/ghost.rs`; ADR 0010.

## M-042 — Ghost install wrapped printables in piped interactive Bash

- Discovered: 2026-08-17
- Status: Fixed
- Failed assumption: `$-` containing `i` was enough to treat the session as an
  editor that can wrap `self-insert`.
- Impact: `bash -i < corpus` with inherited `MBX_GHOST=1` wrapped letters.
  Prefix matches from earlier corpus lines leaked into later `MBX_TEST`
  markers and failed the compatibility comparison.
- Correction: ghost install requires a tty on stdin. The smoke corpus pins
  `MBX_GHOST` and `MBX_HISTORY` off so parent environment cannot enable it.
- Prevention: opt-in editor wrapping must require a tty, not only
  interactive `$-`. Semantic corpus tests must pin feature flags they do not
  intend to exercise.
- Evidence: `_mbx_ghost_install` in `bash/ghost.bash`; `tests/bash/smoke.bash`.

## M-043 — Appending a PTY test truncated a prior assert

- Discovered: 2026-08-18
- Status: Fixed
- Failed assumption: replacing a function's trailing `exit_and_wait` plus
  closing brace was unique enough to insert a new test.
- Impact: the older-match `assert_eq!` in the ghost cycling PTY test lost its
  message and closing delimiter, so `mbx-pty` test `ghost` failed to compile.
- Correction: restore the full assertion, then append the remaining-printables
  test after the complete function.
- Prevention: when inserting after a test, include a unique assertion message
  in the match context, not only a repeated `exit_and_wait` trailer.
- Evidence: `ctrl_x_ctrl_n_cycles_to_older_prefix_match` in
  `crates/pty/tests/ghost.rs`; compile failure on `dc4d78d`.

## M-044 — Ghost Enter armed flag survived a partial keymap disarm

- Discovered: 2026-08-18
- Status: Fixed
- Failed assumption: `_mbx_ghost_disarm_enter` could return after a successful
  emacs disarm and a failed vi-insert disarm without clearing
  `_MBX_GHOST_ENTER_ARMED`, and wrapping printables before Enter helpers was
  safe because helper `can_wrap` had already succeeded.
- Impact: emacs Enter could already be stock `accept-line` while the armed
  flag stayed set, so the next arm skipped and a suffix could be submitted.
  If printables wrapped and a later helper bind failed, `_MBX_GHOST_BOUND`
  stayed 0, `arm_enter` was a no-op success, and stock Enter executed the
  unaccepted suffix.
- Correction: disarm always clears `_MBX_GHOST_ENTER_ARMED` after attempting
  both keymaps. Emacs install binds kill-line / accept-line helpers before
  printables. When `_MBX_GHOST_BOUND=1`, `_mbx_ghost_show` keeps a suffix only
  if Enter is actually armed.
- Prevention: never show an inline suffix unless the Enter macro is armed.
  Bind accept helpers before wrapping `self-insert`. Armed flags must clear
  even when a secondary keymap bind fails.
- Evidence:   `_mbx_ghost_disarm_enter` / `_mbx_ghost_install` /
  `_mbx_ghost_show` in `bash/ghost.bash`; module contract for partial disarm
  in `tests/bash/modules.bash`.
- Recurrence (2026-08-31): `_mbx_highlight_disarm_enter` cleared
  `_MBX_HIGHLIGHT_ENTER_ARMED` only on full success, repeating this exact
  cause. Highlight now always clears the flag after attempting both keymaps;
  module contract covers the partial-vi failure. Prevention stands: when a
  new feature copies an existing arm/disarm pair, diff the two functions
  (M-066). ADR 0015 deleted the highlight Enter arm/disarm path entirely, so
  this recurrence cannot happen there; ghost still has the contract.

## M-045 — Protocol frame reader rejected multi-line search output

- Discovered: 2026-08-16
- Status: Fixed
- Failed assumption: `_mbx_read_bounded_response` could collect sidecar search
  lines the same way it collects one MBX1/MBX2 frame, and a later read would
  still see a second queued frame.
- Impact: a helper that printed two command lines delivered both LFs in one
  `read`, so the protocol reader rejected the buffer (`before_lf` still
  contained a newline). Bounded cycling never left the first snapshot. The
  same over-read rejected a delayed stale QUERY RESULT sitting in the coprocess
  pipe ahead of the current generation's RESULT, so overlapping ghost skip
  never applied the matching suffix.
- Correction: search uses a one-line `read -r` helper that stops at the first
  LF and leaves later lines in the pipe. Ghost QUERY RESULT skip uses that
  line reader on the coprocess FD instead of `_mbx_read_bounded_response`.
- Prevention: protocol frame readers are single-payload and may over-read the
  next queued frame. CLI output and pipelined RESULT frames that are one
  record per line need a line reader plus a focused two-frame contract.
  Do not reuse `_mbx_read_bounded_response` outside a true 1:1 exchange.
- Evidence: `_mbx_search_read_line` in `bash/search.bash`, the two-line
  cycle contract in `tests/bash/modules.bash`, `_mbx_ghost_query_wire` in
  `bash/ghost.bash`, and `overlapping_delayed_result_is_rejected` in
  `crates/pty/tests/ghost.rs`.

## M-046 — Cwd-scoped prefix SQL omitted schema v3 columns

- Discovered: 2026-08-25
- Status: Fixed
- Failed assumption: cherry-picking `exact_prefix_in_cwd` from a pre-HIST-010
  branch could keep that slice's ten-column `SELECT` list.
- Impact: `query()` reads `repo_root` at index 10. The cwd-prefix statements
  stopped at `user`, so `queries_return_bounded_recent_prefix_and_cwd` failed
  with `Invalid column index: 10`.
- Correction: prefix SQL builders interpolate `HISTORY_COLUMNS` so cwd/global
  prefix queries cannot drift from `query()`'s mapped indexes.
- Prevention: any new history `SELECT` must use `HISTORY_COLUMNS` or list every
  mapped index. Rebasing a query helper across a schema version requires a
  focused read of `query()`.
- Evidence: `exact_prefix_sql` / `exact_prefix_cwd_sql` in
  `crates/cli/src/storage.rs` and
  `storage::tests::queries_return_bounded_recent_prefix_and_cwd`.

## M-047 — Ghost Up/Down stripped only the first history-list digit

- Discovered: 2026-08-26
- Status: Fixed
- Failed assumption: `${entry#*[0-9]}` removed the `history` list number the
  same way `_mbx_history_parse_latest` does.
- Impact: after ten or more session entries, `history 12  echo …` became
  `2  echo …` in `READLINE_LINE`. Enter executed that corrupted text.
- Correction: ghost uses the same `number` + two-space separator parse as the
  sidecar recorder.
- Prevention: any `history` list-line parser must match the recorder regex and
  have a two-digit list-number contract. Do not strip a single digit.
- Evidence: `_mbx_ghost_history_entry` in `bash/ghost.bash` and the 12-entry
  module contract in `tests/bash/modules.bash`.

## M-048 — MBX2 ERROR echoed untrusted kind text and dropped correlation

- Discovered: 2026-08-26
- Status: Fixed
- Failed assumption: embedding the raw third field in `unknown MBX2 kind: …`
  and answering a missing history handler with `request_id=0` was a typed
  protocol error.
- Impact: a near-max unknown kind produced an ERROR frame over 64 KiB, so
  `write_message` aborted the serve loop. Bash ERROR decode requires the
  request id, so a disabled-history peer could not correlate. Free-form kinds
  also contradicted the protocol allowlist.
- Correction: unknown kinds map to `unsupported` without echoing. Parse and
  storage failures map to `invalid` / `storage` / `queue_full`. Missing
  handler echoes the client id. ERROR kinds are escaped and bounded.
- Prevention: never interpolate untrusted protocol fields into ERROR kinds.
  Encode ERROR only from the documented allowlist; prove an oversized unknown
  kind stays under `MAX_MESSAGE_BYTES` and still correlates.
- Evidence: `mbx2_error_kind` / `encode_mbx2_error` in
  `crates/cli/src/history_service.rs`; `mbx2_request_id` in
  `crates/cli/src/transport.rs`; `unknown_kind_is_unsupported_and_does_not_echo`
  and `mbx2_without_history_handler_fails_closed`.

## M-049 — History-search helper left monitor/notify enabled

- Discovered: 2026-08-26
- Status: Fixed
- Failed assumption: process-substitution `$!` wait/kill was enough for a
  `bind -x` sidecar lookup, matching the ghost CLI helper.
- Impact: interactive Bash defaults to monitor mode. A search chord could emit
  job-control noise into the editing buffer, unlike ghost which already
  suspends `set -m` / `set -b`.
- Correction: `_mbx_search_helper` saves and restores monitor/notify around
  the helper the same way `_mbx_ghost_query` does.
- Prevention: every sourced `bind -x` process substitution must suspend
  monitor/notify and restore them on every return path. Add a restore
  contract next to the helper.
- Evidence: `_mbx_search_helper` / `_mbx_search_restore_jobs` in
  `bash/search.bash` and the monitor-restore contract in
  `tests/bash/modules.bash`.

## M-050 — Search and editor inserted C0 bytes into the line buffer

- Discovered: 2026-08-26
- Status: Fixed
- Failed assumption: sidecar search matches and `MBX_EDITOR_INSERT_TOKEN` were
  already ordinary command bytes, so only ghost suffixes needed a C0/DEL gate.
- Impact: an ESC in helper stdout or the editor token became `READLINE_LINE`.
  Readline redisplay could then inject terminal controls. The chord still did
  not `eval` the text.
- Correction: `_mbx_text_has_c0_or_del` is shared. Search skips C0/DEL lines;
  a snapshot of only hostile rows leaves the typed line. The editor token is a
  no-op when it contains C0/DEL. Ghost still rejects a control suffix.
- Prevention: every write of untrusted text into `READLINE_LINE` must use the
  C0/DEL gate. Cover a mixed ESC-then-clean helper and an ESC-only helper.
- Evidence: `_mbx_text_has_c0_or_del` in `bash/protocol.bash`;
  `_mbx_search_helper` / `_mbx_editor_insert_token`; H-6/H-7 in
  `tests/bash/modules.bash` and `docs/hrd-002-hostile-audit-plan.md`.
- Recurrence (2026-08-31): ghost Up/Down history-motion
  (`_mbx_ghost_previous_history` / `_mbx_ghost_next_history`) and ranked
  completion insert (`_mbx_comp_apply_word_token`) wrote untrusted text into
  `READLINE_LINE` with no C0/DEL gate. Overlay display already sanitized;
  insert did not. Both sinks now gate; module contracts skip a C0 history
  row and refuse a C0 candidate token. Prevention stands: every
  `READLINE_LINE` assignment of untrusted text uses `_mbx_text_has_c0_or_del`.

## M-051 — Engine coprocess died on prompt Ctrl+C

- Discovered: 2026-08-26
- Status: Fixed
- Failed assumption: a session-lifetime `coproc` listed as job #1 would stay
  out of the foreground process group, so only `bind -x` helpers needed
  monitor/notify isolation (M-049).
- Impact: Ctrl+C at the prompt SIGINT'd `mbx serve`. Bash printed
  `[1]+ Interrupt coproc _MBX_ENGINE_COPROC`, which stole the next command's
  first byte (`rintf` instead of `printf`) and left the helper dead.
- Correction: `_mbx_engine_start` suspends monitor/notify only around spawn,
  ignores INT/QUIT/TSTP in the coproc subshell so `exec` inherits SIG_IGN,
  disowns the child, and restores the caller's flags. TERM/KILL shutdown and
  helper-crash fallback are unchanged.
- Prevention: long-lived interactive children must not remain monitored jobs
  and must ignore terminal interrupt signals across `exec`. Prove SIGINT
  survival plus monitor restore next to engine start, and assert PTY Ctrl+C
  does not print coproc job noise.
- Evidence: `_mbx_engine_start` in `bash/engine.bash`; M-051 contract in
  `tests/bash/modules.bash`; `next_prompt_usable_after_insert_and_ctrl_c` in
  `crates/pty/tests/editor_bind_x.rs`.

## M-052 — Highlight install reported bound with no widgets

- Discovered: 2026-08-27
- Status: Fixed
- Failed assumption: `_mbx_highlight_bind_x` could treat stock `bind -p`
  occupancy as a hard skip, so `self-insert` blocked every printable wrap while
  `_MBX_HIGHLIGHT_BOUND=1` (M-040 recurrence).
- Impact: `MBX_HIGHLIGHT=1` appeared installed but `bind -X` listed no
  `_mbx_highlight_*` widgets; highlighting was a no-op.
- Correction: wrap occupancy now matches `_mbx_ghost_can_wrap`; `_MBX_HIGHLIGHT_BOUND=1`
  only when `bind -X` lists `_mbx_highlight_self_insert` and Enter can arm.
- Prevention: PTY H-1 must assert both `_MBX_HIGHLIGHT_BOUND=1` and a highlight
  widget in `bind -X`.
- Evidence: `bash/highlight.bash`; `highlight_install_sets_bound_flag_and_wraps_self_insert`
  in `crates/pty/tests/highlight.rs`; H-1 in `docs/hlt-comp-review-close-plan.md`.

## M-053 — Highlight gate rejected every styled line for C0

- Discovered: 2026-08-27
- Status: Fixed
- Failed assumption: `_mbx_text_has_c0_or_del` on the full styled helper output
  was a safe gate before assigning `READLINE_LINE`.
- Impact: SOH/STX/SGR markers in styled output failed the gate; refresh always
  fell back to plain text and highlighting never activated.
- Correction: validate helper output with strip-then-compare against
  `_MBX_HIGHLIGHT_PLAIN`; reject only unexpected C0 in the stripped remainder.
- Prevention: module H-2 and refresh tests must accept marker stubs only when
  strip equals plain.
- Evidence: `_mbx_highlight_validate_styled` in `bash/highlight.bash`;
  `tests/bash/modules.bash`.

## M-054 — Highlight strip index used string concatenation

- Discovered: 2026-08-27
- Status: Fixed
- Failed assumption: `index+=2` in a sourced Bash module increments an integer
  by two.
- Impact: Bash treated `index` as a string (`5` + `2` → `52`), breaking SGR
  skipping in `_mbx_highlight_strip_line`.
- Correction: use `index=$((index + n))` for numeric advances.
- Prevention: never use `+=` with numeric counters in sourced Bash modules unless
  both sides are guaranteed arithmetic context.
- Evidence: `_mbx_highlight_strip_line` in `bash/highlight.bash`.

## M-055 — Highlight refresh clobbered helper payload in REPLY

- Discovered: 2026-08-27
- Status: Fixed
- Failed assumption: `_mbx_wait_child_until` could run before the styled payload
  was copied out of `REPLY`.
- Impact: styled helper output was lost after the child wait, matching the
  search-root failure mode.
- Correction: save styled line and point to locals before
  `_mbx_wait_child_until`; suspend monitor/notify around the helper (M-049).
- Prevention: every highlight/search helper read must copy `REPLY` before waiting
  on `$!` and must restore `$-` job flags.
- Evidence: `_mbx_highlight_refresh` in `bash/highlight.bash`; H-5 in
  `tests/bash/modules.bash`.

## M-056 — `[[ ]]` glob `*` is not a Kleene star on a character class

- Discovered: 2026-08-28
- Status: Fixed
- Failed assumption: `[[ $name == [A-Za-z_][A-Za-z0-9_]* ]]` checked that every
  remaining character was alphanumeric or underscore.
- Impact: a wrap answer of `git;rm` matched the glob (`*` consumes `;rm`) and
  would have been written into `config.bash`.
- Correction: `scripts/configure.bash` validates wrap names with
  `=~ ^[A-Za-z_][A-Za-z0-9_-]*$`.
- Prevention: user-supplied identifiers and command names must use anchored
  `=~` regexes, not `[[ == ]]` globs, when `*` would otherwise match leftover
  bytes.
- Evidence: `sanitize_wrap` in `scripts/configure.bash`; smoke wrap-token
  reject in `tests/bash/smoke.bash`.
- Recurrence (2026-08-31): `_mbx_comp_identifier_ok` used the same
  `[[ $1 == [A-Za-z_][A-Za-z0-9_]* ]]` glob, so `git;rm` matched. It now uses
  an anchored `=~ ^[A-Za-z_][A-Za-z0-9_-]*$` (hyphens allowed for command
  names). Module contract rejects `git;rm` and `foo bar`.

## M-057 — Isolated HOME tests inherited XDG_CONFIG_HOME

- Discovered: 2026-08-28
- Status: Fixed
- Failed assumption: `env HOME=$tmpdir` was enough to isolate install/configure
  writes in CI.
- Impact: GitHub Actions CI failed with `install --no-build must write
  ~/.config/mbx/config.bash` because `config_path` honored the runner's
  absolute `XDG_CONFIG_HOME` and wrote outside the temp HOME.
- Correction: smoke install/configure cases set `XDG_CONFIG_HOME=` and
  `XDG_DATA_HOME=` via `iso`.
- Prevention: any test that supplies a fake `HOME` must also clear or override
  `XDG_CONFIG_HOME` and `XDG_DATA_HOME` when the code under test reads those
  variables.
- Evidence: `iso` in `tests/bash/smoke.bash`; CI run
  https://github.com/ishitvagoel/ColorBash/actions/runs/33132959261

## M-058 — Configure menu did not round-trip the saved file

- Discovered: 2026-08-28
- Status: Fixed
- Failed assumption: opening choice “keep current answers” preserved in-memory
  defaults, so re-running the menu or `mbx_configure --answers` did not need
  to parse `~/.config/mbx/config.bash`.
- Impact: a second interactive run started from empty/preset values and wiped
  customizations; `mbx_configure --answers KEY=value` reset every other flag.
- Correction: `load_existing_config` sources the file in a subprocess and
  maps `MBX_*` into answers keys. Interactive runs auto-load when the file
  exists. `mbx_configure` prepends `--from-config`.
- Prevention: any tool that writes a user config must load that file as the
  default starting state on re-entry; assert a piped `4` then `w` after a
  comfort write keeps ghost, and `--from-config --answers` overlays wrap.
- Evidence: `load_existing_config` in `scripts/configure.bash`;
  `mbx_configure` in `bash/config.bash`; smoke cases in
  `tests/bash/smoke.bash`.

## M-059 — Configure `--build` was parsed and ignored

- Discovered: 2026-08-28
- Status: Fixed
- Failed assumption: documenting `--build` / `--no-build` on
  `scripts/configure.bash` was enough because `install.bash --interactive`
  already compiled the helper.
- Impact: `bash scripts/configure.bash --build` never invoked cargo, so a
  user who skipped `install.bash` could save options against a missing
  helper with no build attempt.
- Correction: `maybe_build` runs `cargo build --release --workspace` from
  the repo root when `NO_BUILD=0`. Default for this script stays
  `--no-build`.
- Prevention: a parsed CLI flag that names a subprocess must have a
  production call plus a test that the missing executable fails before
  writing output.
- Evidence: `maybe_build` in `scripts/configure.bash`; smoke
  `--build` without cargo.


## M-060 — A permission test assumed CAP_DAC_OVERRIDE could not be present

- Discovered: 2026-08-29
- Status: Fixed
- Failed assumption: `unreadable_store_fails_closed_without_widening` treated
  a successful open of a mode-`0000` file as impossible, so it panicked
  instead of asserting an invariant.
- Impact: root (or any caller with `CAP_DAC_OVERRIDE`, the default uid in many
  container images) bypasses the mode-`0000` denial at the kernel level, so
  `QueuedHistoryStore::open` succeeds there. `bash tests/run.bash` failed at
  `cargo test --workspace` on such a host, before Clippy or any Bash suite
  ran, so the canonical suite could not be completed in that environment at
  all.
- Correction: the test now mirrors its sibling
  `restrictive_file_is_not_made_more_permissive` — it accepts either outcome
  and asserts the invariant that actually holds under both: the on-disk mode
  is never widened by our own code.
- Prevention: a test that asserts an OS-level permission denial must accept
  a privileged caller's outcome too, and assert the narrower invariant (no
  widening) rather than the open's success or failure.
- Evidence: `crates/cli/src/storage.rs`
  (`unreadable_store_fails_closed_without_widening`); passes identically as
  uid 0 and as a non-root user.

## M-061 — A relative helper path stopped resolving after an intentional `cd`

- Discovered: 2026-08-29
- Status: Fixed
- Failed assumption: a `$MBX_TEST_BIN` argument could stay relative for the
  whole script, because most callers only used the absolute default.
- Impact: `tests/integration/protocol.bash target/debug/mbx`, the exact
  invocation `README.md` documents, failed with "No such file or directory"
  — one case in the script `cd`s into a directory it then deletes, and a
  relative path no longer resolves from there. Only `tests/run.bash`, which
  always passes an absolute path, had ever exercised this script.
- Correction: the script resolves a relative `$MBX_TEST_BIN` against the
  invocation `$PWD` immediately after reading it, before any `cd`.
- Prevention: a test harness that both accepts a relative path argument and
  changes directory mid-run must resolve that argument to an absolute path
  first, and the documented invocation must itself be run as a regression
  case.
- Evidence: `tests/integration/protocol.bash`; passes with both a relative
  and an absolute `target/debug/mbx` argument.

## M-062 — Highlight color was decided from the helper's own stdout, which is never a terminal

- Discovered: 2026-08-29
- Status: Fixed (2026-08-31)
- Failed assumption: `crate::environment::color_disabled_for_stdout()`
  (an `io::stdout().is_terminal()` check on the `mbx` process itself) was a
  valid way to decide whether to style `mbx highlight`'s output.
- Impact: every production caller of `mbx highlight` — the process
  substitution spawn (`exec {fd}< <(exec "$MBX_BIN" highlight ...)`) and the
  MBX2 coprocess added by ADR 0014 alike — has the mbx process's own stdout
  connected to a pipe, never the interactive terminal. `color_disabled_for_stdout()`
  therefore always returned true in real use, so `MBX_HIGHLIGHT=1` silently
  never styled anything in a live session, despite passing every existing
  test (none asserted color was actually present) and being carried as
  `validation` in the roadmap. Only a user manually running
  `"$MBX_BIN" highlight ...` directly at their own terminal (the README's
  demo invocation) ever saw real color, because that invocation's stdout
  genuinely is the terminal.
- Correction: `mbx highlight` gained an explicit `--color 0|1` (and the
  MBX2 `HIGHLIGHT` frame an explicit `color` field) that a caller who can see
  the real terminal — Bash, via the single `_mbx_color_capable` predicate
  already used for prompt flags — passes explicitly, instead of the helper
  guessing from its own stdout. `_mbx_color_capable` was factored out of
  `_mbx_prompt_flags` in `bash/config.bash` as the one place this decision is
  made. The pre-existing stdout-tty check remains only as the *default* for a
  direct manual CLI invocation with no `--color` given.
- Why `Mitigated` and not `Fixed` (superseded 2026-08-31): the plumbing was
  correct, but live refresh still passed `color=0` until ADR 0015. That
  remaining half is now fixed: `_mbx_highlight_color_flag` decides color from
  TERM/NO_COLOR/MBX_COLOR plus a writable controlling tty (`-t 1` is false
  inside `bind -x` widgets, which was the same class of bug on the Bash side).
  Evidence: `crates/pty/tests/highlight.rs`
  (`highlight_preview_row_paints_sgr_below_an_intact_prompt`).
- Prevention: a capability decision (terminal color, width, TTY-ness) must
  come from whichever side of an IPC boundary can actually observe it, passed
  explicitly as data — never re-derived by inspecting the other side's own
  process state. For `bind -x` widgets, observe `/dev/tty` or stdin, not
  stdout. This is the same rule ADR 0007/`PRM-007` already apply to
  prompt rendering; highlighting had quietly violated it.
- Evidence: `crates/cli/src/cli.rs` (`--color` parsing),
  `crates/cli/src/app.rs::execute_highlight`, `bash/config.bash`
  (`_mbx_color_capable`), `docs/adr/0014-highlight-over-coprocess.md`.

## M-063 — `set +m` does not suppress a background job's start announcement from a keystroke callback

- Discovered: 2026-08-29
- Status: Fixed
- Failed assumption: `_mbx_engine_write`'s and `_mbx_engine_exchange`'s
  `( trap '' PIPE; printf ... ) &` background write was adequately silenced
  for an interactive session because the caller (ghost, and now highlight)
  already wraps the whole call in `set +m; ...; set -m`.
- Impact: confirmed with a minimal reproduction (`set +m; ( sleep 0.2 ) &`
  under a real PTY) that Bash still prints `[N] PID` for a backgrounded job
  regardless of monitor mode, when the backgrounding happens inside a
  `bind -x` self-insert callback — the announcement goes to the shell's own
  stderr, not the command's. The same call from `PROMPT_COMMAND` does not
  print it; only the keystroke-callback context does. Every wire-path
  keystroke therefore leaked a `[N] PID` line into the terminal, corrupting
  the redraw and, in one PTY test, cascading into repeated redraws that blew
  through the test's timeout. This affected ghost's existing wire path too,
  not only highlighting's new one — it had simply never been exercised
  densely enough (or checked for this specific artifact) to be caught.
- Correction: wrap the whole backgrounding statement's stderr, not rely on
  job-control mode: `{ ( trap '' PIPE; printf ... ) & } 2>/dev/null`. `$!`
  still resolves to the correct PID through the group. Applied identically in
  `_mbx_engine_write` and `_mbx_engine_exchange`.
- Prevention: `set +m` suppresses job-control *behavior* (SIGTSTP handling,
  foreground/background switching), not the interactive shell's `[N] PID`
  start announcement for a bare `&`; suppressing that announcement requires
  redirecting the backgrounding statement's own stderr. Any future background
  job started from a `bind -x` callback must be checked under a PTY with
  monitor mode considered untrustworthy for this purpose.
- Evidence: `bash/engine.bash` (`_mbx_engine_write`, `_mbx_engine_exchange`);
  `crates/pty/tests/ghost.rs` full suite green and faster afterward (65.18s →
  21.61s, consistent with removing spurious redraw cascades);
  `crates/pty/tests/highlight.rs` full suite.

## M-064 — Readline does not treat `\001`/`\002` as invisible inside `READLINE_LINE`

- Discovered: 2026-08-29
- Status: Fixed (2026-08-31, ADR 0015)
- Failed assumption: ADR 0013 assumed Readline's non-printing markers
  (`RL_PROMPT_START_IGNORE`/`_END_IGNORE`, `\001`/`\002`) make an enclosed SGR
  run zero-width and invisible wherever they appear, because that is their
  documented behavior inside `PS1`.
- Impact: they do not have that effect when they appear inside the *edit
  buffer* (`READLINE_LINE`, as `bind -x` highlighting and ghost both use).
  Confirmed empirically and at the byte level (not merely inferred): once
  M-062's color-detection bug was fixed enough to let real styled bytes reach
  `READLINE_LINE`, Bash's own redisplay rendered the markers and the CSI
  escape they wrap using its ordinary unprintable-control-character
  convention — literal, visible `^A`, `^[`, `^B` two-character sequences —
  instead of hiding them. This means opt-in syntax highlighting's live
  interactive path has never displayed real color correctly since its
  original implementation; the defect was masked by M-062 (color was always
  off in practice) and by every existing test asserting only byte-exactness
  or plain-mode round-tripping, never that genuine color rendered
  correctly on screen.
- Current state: ADR 0015. Live refresh sends a real color decision and
  paints SGR on the reserved preview row. `READLINE_LINE` stays plain.
- Prevention: a technique's documented behavior for one call site (`PS1`)
  must not be assumed to transfer to a structurally different call site
  (`READLINE_LINE`) without a PTY test that actually captures and asserts the
  rendered bytes contain no leftover control characters — not just that they
  round-trip back to the plain buffer.
- Next step (done 2026-08-31): ADR 0015 paints styled bytes on one reserved
  row below the prompt (M-065 IND/DECSC). `READLINE_LINE` stays permanently
  plain; the Enter restore macro is gone. PTY + `Screen` evidence:
  `highlight_preview_row_paints_sgr_below_an_intact_prompt`.
- Evidence: byte-level capture and analysis over a live PTY session (raw
  `0x01`/`0x1b`/`0x02` confirmed present via `od -c` on `mbx highlight`'s own
  output, contrasted with the literal two-character `^A`/`^[`/`^B` sequences
  Bash's redisplay produced for the same bytes once inserted into
  `READLINE_LINE`); ADR 0015; `crates/pty/tests/highlight.rs`
  (`typed_line_renders_without_caret_control_leftovers`, added 2026-09-03,
  pins the ADR 0015 invariant that the typed line never shows caret-encoded
  markers).

## M-065 — Completion overlay's DECSC/DECRC save is invalidated by its own scroll

- Discovered: 2026-08-29
- Status: Fixed (2026-08-30)
- Failed assumption: ADR 0013's completion overlay assumed `\e7` (DECSC,
  save cursor) before drawing up to eight rows below the prompt, then `\e8`
  (DECRC, restore) and `\e[J` (erase to end of screen) to hide it, was
  terminal-safe. DECSC/DECRC save an *absolute* screen position.
- Impact: reproduced with a genuine PTY session and a purpose-built VT
  screen model (`crates/pty/src/screen.rs`): at a 6-row terminal with the
  prompt a couple of lines down — an entirely ordinary state, not a
  contrived edge case — showing an eight-candidate overlay draws enough
  lines to scroll the screen. `\e8` then restores the *pre-scroll* absolute
  coordinates, which no longer correspond to the overlay's origin, and the
  following `\e[J` erases from that stale position. The observable damage is
  that the overlay's own rows are left stranded on screen while the
  scrollback above them is destroyed: the modelled screen reads
  `cand2 cand3 cand4 cand5 cand6` with every earlier line gone. The existing
  overlay PTY suite (`crates/pty/tests/completion_harness.rs`) never caught
  this because every case runs at a fixed 24-row window with the prompt near
  the top, so the draw never scrolls.
- Correction: two changes, both in `bash/completion.bash`.
  1. `_mbx_comp_overlay_reserve` makes room *before* anything saves the
     cursor. `\eD` (IND) moves down a row and scrolls at the bottom margin,
     so `count` of them let the screen absorb the scroll the draw was going
     to cause; moving back up `count` rows lands on the prompt's row wherever
     it now is (if the screen scrolled by `s`, the cursor is at `L - count`
     and the prompt moved to `R - s`, which are the same row). A `\e7` taken
     after this cannot be invalidated. IND rather than `\n` specifically
     because IND leaves the column alone — `\n` would save the start of the
     prompt line instead of the user's cursor within it, and the dismissing
     `\e[J` would then erase the prompt text itself.
  2. `_mbx_comp_overlay_capacity` caps the draw at `LINES - 2`. Reserving
     keeps the save valid but does not stop the reservation itself from
     scrolling the prompt off the top: eight rows do not fit under a prompt
     on a six-row terminal. With `k` drawn rows the prompt lands on `L - k`,
     so `k <= L - 2` keeps it and one line of context on screen.
  No DSR (`\e[6n`) round trip was needed after all. The earlier note that a
  fix required one was wrong: it assumed the cursor's row had to be *known*,
  when it only had to be made *safe*. Avoiding DSR also avoids its real
  costs — a timeout on terminals that do not answer, and the risk of
  swallowing type-ahead while reading the reply.
- Trap found while validating: "is the prompt still visible" does **not**
  discriminate this bug. Readline redraws the prompt line after a `bind -x`
  widget returns, so it comes back either way, and a test asserting only that
  passes against the unfixed code. The property that separates them is
  whether the overlay's own rows were actually erased.
- Prevention: DECSC/DECRC (or any technique that captures an absolute screen
  position) must not bracket output whose own length can push the cursor past
  the bottom of the terminal. Either reserve the space first, so the scroll
  happens before the save, or cap the output to the space that exists. And
  when writing the test, assert the state that only the defect can produce —
  not the state a later redraw will restore regardless.
- Follow-up defect introduced by the fix, caught in review before merge:
  capping the *draw* without capping the *selection* let navigation and
  acceptance address rows that were never on screen. With eight candidates on
  a six-row terminal only four rows are drawn, but `_mbx_comp_cycle_next` and
  `_mbx_comp_cycle_prev` still advanced modulo all eight, so past index 3
  nothing was highlighted and `_mbx_comp_accept_ranked` would insert a
  candidate the user had never seen — a direct violation of the project's
  central promise that nothing is inserted the user did not choose.
  `_MBX_COMP_OVERLAY_SHOWN` now records the drawn count on every path
  (computed before the tty branch, so it means the same thing whether or not
  this process owns a terminal) and bounds both cycling and acceptance.
  Covered by `tests/bash/modules.bash` OV-3, whose accept case needed a
  matching ranked snapshot to reach the insertion at all — without it the
  eligibility gate refused for an unrelated reason and the assertion was
  vacuous, passing against both the fixed and unfixed code. A control case
  asserting that a *drawn* row does insert now guards that.
- Evidence: `bash/completion.bash`
  (`_mbx_comp_overlay_reserve`, `_mbx_comp_overlay_capacity`,
  `_MBX_COMP_OVERLAY_SHOWN`);
  `crates/pty/tests/overlay_screen.rs`
  (`overlay_near_the_bottom_of_a_short_terminal_leaves_the_prompt_intact`, no
  longer `#[ignore]`d — it fails against the unfixed code listing the five
  stranded rows, and passes with the fix); `crates/pty/src/screen.rs` gained
  IND/RI so the model can represent the fix; `tests/bash/modules.bash` OV-2
  covers the capacity clamp including a nonsensical `LINES`.

## M-066 — Highlight's coprocess loop dropped the ACK tolerance its ghost twin has

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `_mbx_highlight_refresh_wire` (added for `HLT-004`, ADR
  0014) was written as a deliberate mirror of `_mbx_ghost_query_wire`, and
  the mirroring was assumed complete because both loops implement the same
  ADR 0011 generation and stale-reply skip. They did not match: ghost's loop
  skips an intervening three-field `ACK` frame and keeps reading, and the
  highlight copy omitted that branch, so any frame that was not a `STYLED`
  fell straight through to `_mbx_engine_stop`.
- Impact: `MBX_GHOST` and `MBX_HIGHLIGHT` are mutually exclusive, but
  `MBX_HIGHLIGHT` and `MBX_HISTORY` are not — both features read the one
  coprocess fd. A history `RECORD` whose `ACK` was still queued when a
  keystroke landed mid-cycle would be read by the highlight loop, fail to
  parse as `STYLED`, and tear down a perfectly healthy helper. The visible
  result is highlighting silently degrading to plain text and the next
  prompt cycle paying a fresh helper spawn, for a condition the transport
  was explicitly designed to tolerate.
- Correction: added the same ACK-skip branch ghost uses, with a comment
  naming why the two features share the fd. The non-ACK case still stops the
  engine, so the fix does not widen into "ignore every unexpected frame".
- Prevention: when a new feature copies an existing wire loop, diff the two
  loops rather than re-deriving them — the branches that look like defensive
  padding (ACK tolerance here) are the ones that encode a real, already-paid
  lesson. Two loops reading the same fd must agree about every frame kind
  that can appear on it.
- Evidence: `bash/highlight.bash` (`_mbx_highlight_refresh_wire`);
  `tests/bash/modules.bash` H-6 (a queued ACK is skipped and does not stop
  the engine) and H-7 (an unexpected non-ACK frame still stops it). Both
  were confirmed to fail against the unfixed code before the fix landed.

## M-067 — `mbx repo root` output was trusted without checking the child's exit status

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `_mbx_search_repo_root` (added for the `SRCH-003` repo
  filter) assumed a non-empty first line of output was sufficient evidence
  that the helper had resolved a repository root, because `mbx repo root`
  prints nothing and exits nonzero outside a repository. The function did
  capture the child's exit status into a local, then never read it.
- Impact: the guarded case — no repository — happened to work, because the
  helper writes nothing there. The unguarded case is a child that is killed
  or times out after emitting a partial first line: `_mbx_search_read_line`
  returns that fragment, and the search would then scope history to a
  truncated path, silently returning the wrong rows instead of falling
  through to the cwd and recent tiers. A `status` local that is assigned and
  never read is the tell.
- Correction: gate acceptance on `((status == 0))` before the non-empty
  check, so only a helper that actually exited cleanly can scope the search.
- Prevention: a spawned helper's exit status is part of its answer, not
  optional metadata — read it, or do not capture it. Treat an assigned-but-
  unread status variable in a Bash helper wrapper as a defect, not lint
  noise.
- Evidence: `bash/search.bash` (`_mbx_search_repo_root`);
  `tests/bash/modules.bash` R-3 (a helper that prints a plausible root but
  exits nonzero must fall through to cwd), confirmed to fail against the
  unfixed code.
- Recurrence (2026-08-31): `_mbx_highlight_refresh_cli` discarded the
  child's wait status (`>/dev/null`) and returned 0 with the payload even
  after a kill; `_mbx_ghost_query_cli` likewise ignored wait failure if any
  candidate had been parsed. Both now go through `_mbx_wait_or_kill_child`;
  highlight requires exit status 0, and ghost discards candidates from a
  timed-out child. Module contract: a highlight helper that prints two
  well-formed lines then `exit 1` must not install them.

## M-068 — A declared workspace license shipped with no license text in the tree

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `Cargo.toml` has declared `license = "MIT OR
  Apache-2.0"` since the workspace was created, and `.github/workflows/
  release.yml` (added for `REL-001`) was written to package `LICENSE-MIT`
  and `LICENSE-APACHE` into every release tarball on the assumption those
  files existed. Neither file was ever in the repository.
- Impact: the packaging step used `cp ... 2>/dev/null || true`, so the
  missing files were swallowed and the release tarball would have shipped
  binaries under a declared dual license with no license text — a real
  distribution defect that the workflow's own error suppression was hiding.
  Nothing would have failed loudly at tag time.
- Correction: added `LICENSE-MIT` and `LICENSE-APACHE` matching the license
  already declared in `Cargo.toml`, and removed the error suppression from
  the packaging `cp` so a future missing file fails the release build
  instead of silently producing an incomplete tarball. The copyright line
  reads "The ColorBash Authors"; the repository owner should replace it if
  they want a different holder named.
- Prevention: `2>/dev/null || true` on a packaging step converts a missing
  deliverable into a silent one. Suppress errors only where the failure is
  genuinely expected and harmless, and never on the step that assembles what
  ships.
- Evidence: `LICENSE-MIT`, `LICENSE-APACHE`,
  `.github/workflows/release.yml`.

## M-069 — A parsing test was gated on a 50 ms wall-clock budget it could not control

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `provider::tests::context_returns_root_and_branch_for_a_worktree`
  (and two sibling tests that also drive the real `git` binary) asserted that
  `GitRepositoryStatusProvider` reads back the right repository root and
  branch. Because the provider hard-clamps every Git acquisition to
  `MAX_GIT_DEADLINE` (50 ms) — a product invariant that
  `configured_deadline_is_clamped_to_fifty_milliseconds` asserts on purpose,
  and that no caller can raise — each of these tests was *also*, silently,
  an assertion that the machine running it can fork and exec `git` twice
  inside 50 ms. That held on every developer machine it was written on.
- Impact: it does not hold on a shared CI runner. On the first push of this
  branch the test timed out in the `Canonical suite (stable)` job and passed
  in the `MSRV (Rust 1.85.0)` job — same commit, same runner image, opposite
  results, which is direct evidence the failure is machine-speed dependent
  and not a code regression. Left alone this would have made the canonical
  suite intermittently red for reasons unrelated to any change under review,
  which is the fastest way to teach a team to ignore a red suite.
- Correction: added a `retry_while_timed_out` test helper that retries **only**
  `ProviderErrorKind::Timeout`, bounded at 20 attempts with a 10 ms pause, and
  applied it at the three sites that spawn the real `git`. The product
  deadline is untouched — loosening it was never an option and would have
  destroyed the invariant these tests exist to protect.
- Why this is not "just retrying a flake": the retry is scoped to the single
  error kind that expresses "this machine was slow", and to nothing else. A
  genuine regression — a wrong root, a wrong branch, a malformed-output or
  spawn error — still fails on the first attempt.
  `timeout_retry_helper_retries_only_timeouts` asserts exactly that, so the
  narrowness is evidenced rather than merely intended.
- Prevention: a test whose subject is parsing or correctness must not also be
  an unstated benchmark. When the code under test carries a wall-clock
  deadline, decide explicitly which of the two properties each test is
  measuring, and isolate the timing assertion into a test that says so in its
  name.
- Evidence: `crates/cli/src/provider.rs` (`retry_while_timed_out` and its
  contract test); the diverging stable/MSRV results on commit `57c5957`
  (`actions/runs/33290559060`).

## M-070 — `mbx doctor` reported three of the ten chords MBX installs

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `DIAG-001` advertised per-feature keybinding-collision
  coverage, and the implementation's feature table listed only the three
  opt-in features (`MBX_GHOST`, `MBX_HIGHLIGHT`, `MBX_COMP_OVERLAY`). The
  assumption was that those are the only installers that can decline a chord.
  They are not: history-search insert and restore, insert token, ranked accept,
  and ranked cycle all install whenever the shell is interactive, all decline
  an already-bound chord, and all have their own `*_OVERRIDE` escape hatch.
- Impact: the most likely collisions were invisible. With `\C-xh` already
  bound, `_mbx_search_install` leaves `_MBX_SEARCH_BOUND=0` and doctor printed
  "no opt-in keystroke feature is enabled" — actively misleading, in the one
  command whose entire purpose is to explain why something is not working.
  Two further defects sat in the same section: a feature whose chord was
  declined was attributed to "stdout is not a tty" even for features that
  never test a tty, and the installers that do test one test *stdin*
  (`-t 0`), not stdout, so the explanation named the wrong file descriptor.
  Separately, `MBX_HISTORY=1` with a store whose path resolves but whose row
  count fails printed no diagnostic at all, letting doctor exit zero while
  history capture was unusable.
- Correction: widened the table to all ten chords, added a per-row flag so the
  tty explanation is offered only by the two features that actually gate on
  one and now asks about stdin to match them, distinguished "installer has not
  run" from "chord declined", and made an unreadable history store a `[FAIL]`
  with a fix line.
- Prevention: a diagnostic command's coverage claim is a contract like any
  other and needs a test that enumerates what it must cover. "Reports every
  collision" is not evidenced by a test that only exercises the features the
  author happened to think of.
- Evidence: `bash/config.bash` (`mbx_doctor`); `tests/bash/modules.bash` D-4
  (an always-on installer that declined its chord is named along with its
  override variable) and D-5 (an unreadable store is a `[FAIL]` and a nonzero
  exit). Reported by an automated reviewer on PR #52 and confirmed against the
  installers before fixing.

## M-071 — The release workflow would publish a release from a manual branch run

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `.github/workflows/release.yml` (added for `REL-001`)
  carried `workflow_dispatch` so the pipeline could be smoke-tested without
  cutting a tag, on the assumption that a manual run would exercise the build
  and stop there.
- Impact: both jobs ran. On a `workflow_dispatch` from a branch,
  `GITHUB_REF_NAME` is the branch name, so the publish step would have run
  `gh release create main` — and `gh release create` creates a missing tag
  from the default branch's latest state, so a single manual run would have
  published a bogus release *and* an unintended tag whose commit need not
  match the uploaded binaries. The one action the workflow's own header
  comment promised required "a deliberate, human decision" was reachable by
  the button meant for testing.
- Correction: gated the `publish` job on
  `startsWith(github.ref, 'refs/tags/v')`. `workflow_dispatch` now does what
  it was meant to do — build the matrix and stop — which is exactly what an
  untested pipeline needs.
- Prevention: a workflow with both a tag trigger and a manual trigger must
  state, per job, which triggers that job is for. Any job with an external
  side effect defaults to the narrower trigger.
- Evidence: `.github/workflows/release.yml`. Reported by an automated reviewer
  on PR #52.

## M-072 — A Bash render-deadline test was a benchmark of the host's Bash build

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `tests/bash/modules.bash` asserted that a near-64 KiB
  prompt request against a deliberately stalled coprocess completes in under
  200 000 us, with `MBX_RENDER_TIMEOUT=.10`. The assumption behind that
  constant was that the fixed cost outside the deadline-governed section fits
  in the remaining 100 ms on every supported Bash.
- Impact: the Bash 5.0 CI leg — added by this same branch, so this had never
  been observed before — failed on it, while 5.1 and 5.2 passed. Reproduced
  locally against a from-source Bash 5.0 build. Measuring elapsed minus
  timeout at render timeouts of 50/100/200 ms gave a flat ~121 ms on Bash 5.0
  and a flat ~32 ms on Bash 5.2. The flatness is the finding: **the deadline is
  honored exactly on both versions**, and what differs is only a fixed
  per-version cost that the deadline never governed. The test was failing a
  supported configuration for a property it was not trying to assert, and no
  product defect existed.
- Correction: replaced the single-run wall-clock ceiling with a differential
  measurement — the same stalled request is run at two render timeouts and the
  elapsed times must differ by the timeout difference. The per-version fixed
  cost is identical in both runs and cancels. A generous absolute ceiling still
  catches an unbounded wait, which is what a genuinely broken deadline
  produces, since the stall never returns.
- Why this is not loosening the test: the new form asserts something the old
  one could not — that elapsed time *tracks the deadline*. Sabotaging the
  fixture so the timeout no longer governs (both runs at one budget) makes it
  fail with a delta of -364 us, confirmed before landing.
- Prevention: when a test's subject is that a deadline is honored, assert
  against the deadline, not against a wall-clock constant that also has to
  cover unrelated fixed costs. If a constant is unavoidable, derive it from a
  measurement taken on the same host in the same run.
- Follow-up correction, same day: the first version of the differential ran
  the shorter leg at a 50 ms timeout, below the original 100 ms. That measured
  the deadline correctly but was too short for a 64 KiB request to reach the
  stalled peer at all on Bash 5.0 in a container, so `the fitting near-limit
  request was not sent` failed instead. Both legs now sit at or above the
  original timeout (100 ms and 200 ms). The tolerance window is measured
  rather than guessed: the delta lands near the nominal 100 000 us idle and
  compressed to ~68 000 us with every core saturated, so the window is
  30 000-250 000 us, still far from the ~0 a non-governing deadline produces.
- Follow-up (2026-08-31): stall-until-timeout cases (CRLF lookahead,
  oversized-PWD fallback, per-call helper, fallback chain) now use
  `assert_elapsed_tracks_deadline`. Fast-path and decode-bound ceilings stay
  absolute because they are not stall-until-timeout measurements.
- Evidence: `tests/bash/modules.bash` (`measure_near_limit_prompt`,
  `assert_elapsed_tracks_deadline`); all three Bash suites pass on a
  from-source Bash 5.0 build and on Bash 5.2.

## M-073 — The Bash compatibility corpus compared echoed input as if it were program output

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `tests/bash/smoke.bash` proves MBX does not change Bash's
  semantics by running `tests/bash/corpus.bash` under a plain `bash -i` and
  again under an MBX-initialized one, then comparing every `MBX_TEST:` marker
  with `grep -o`. The assumption was that those markers are program output.
  They were not only that. `bash -i` reading a script from a file echoes input
  lines into the same stream as output, and because the corpus wrote the
  marker prefix as a literal in its own source, `grep -o` captured the echoed
  source lines too — e.g. both `MBX_TEST:subshell=/tmp` (real output) and
  `MBX_TEST:subshell=%s\n' "$PWD")` (the echo of the line that produced it).
- Impact: the comparison silently asserted something it does not name and
  cannot legitimately require — that MBX leaves Bash's *input echo*
  byte-identical — when changing `PS1`/`PS2` and Readline state is precisely
  what MBX is for. It also made the outcome depend on the Readline build: echo
  matched on a vanilla Bash 5.0 and 5.2, and diverged on Ubuntu 20.04's Bash
  5.0, where the MBX run dropped the echo of two corpus lines. CI failed there
  with "Bash corpus semantics changed after MBX initialization" while every
  real result — `process-substitution=test`, `array=alpha,beta`, `status=1` —
  matched exactly. The alarm was on the one invariant this project most needs
  to be trustworthy, and it was false.
- Correction: the corpus now holds its marker prefix in a variable (`M`) and
  interpolates it, so the literal string the suite greps for never appears in
  the corpus source. Echoed input can no longer match the pattern on any
  Readline build, which removes the entire class rather than patching the one
  observed difference. Every construct the corpus covered before is unchanged.
- Prevention: when a test extracts evidence from a captured stream, make the
  evidence impossible to forge from the input that produced it. A marker
  written literally in the script that emits it cannot distinguish "the shell
  echoed my command" from "the program printed this".
- Evidence: `tests/bash/corpus.bash`. The baseline marker set is now 15 lines
  of pure program output on Bash 5.0 with no echoed source; the suite passes
  on a from-source Bash 5.0 and on 5.2; and injecting a real semantic change
  (an rc that alters `HOME` after sourcing `init.bash`) is still caught as
  `-MBX_TEST:variable=/root` / `+MBX_TEST:variable=/hijacked-by-mbx`.

## M-074 — CI ran the whole matrix twice concurrently on every PR commit

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: the CI workflow rewritten for `T0-3` kept a bare `push:`
  trigger alongside `pull_request:`, on the assumption that this simply covers
  both cases. A bare `push:` fires on every branch push, so once a branch has
  an open pull request each commit triggers two complete, concurrent workflow
  runs — two canonical suites, two MSRV jobs, two Bash matrices — competing
  for runners at the same moment. Widening that workflow from one job to six
  in this same change multiplied the cost of the mistake without anyone
  noticing it was there.
- Impact: this repository's PTY suites drive real interactive Bash sessions
  against wall-clock deadlines (`wait_for_count` allows 8 s; `read_until`
  deadlines are similar), so runner contention is not a cosmetic cost — it is
  the thing that makes them fail. On commit `8684622` the two concurrent runs
  disagreed: the canonical suite passed in one while MSRV failed
  `ctrl_p_loads_history_after_dismissing_suffix` in the other, with
  `last=0` — nothing recorded at all in eight seconds, on identical code.
  Superseded runs from earlier pushes were also left running, adding still
  more load to the run whose result anyone would actually read.
- Correction: scoped `push` to the default branch, so pull requests are
  covered once by `pull_request`, and added a `concurrency` group keyed on
  workflow and ref with `cancel-in-progress`, so a new push supersedes the run
  in flight.
- Prevention: `on: push` plus `on: pull_request` is a double-run by default,
  not a belt-and-braces pair. Any repository whose tests are timing-sensitive
  should treat concurrent duplicate runs as a correctness problem rather than
  a billing one, and should set a concurrency group from the start.
- Evidence: `.github/workflows/ci.yml`; the disagreeing pair of runs
  33291533211 and 33291535294 on commit `8684622`.

## M-075 — PTY history cases raced the very budget MBX is designed to abandon

- Discovered: 2026-08-30
- Status: Mitigated (not Fixed — see "What is not established" below)
- Failed assumption: the PTY suites that assert "this command was recorded"
  (`wait_for_count`) treated recording as guaranteed once the prompt returned.
  It is not. MBX deliberately drops a history record rather than let a slow
  helper stall the prompt, so every such case is implicitly racing
  `MBX_HISTORY_TIMEOUT`. The harness already knew this in part — its comment
  says these cases are "tolerant of heavily parallel CI load" and had raised
  the budget from the production 0.10 s to 1.0 s — but 1.0 s is still a race
  when many PTY binaries run at once, which is exactly what
  `cargo test --workspace` does.
- Impact: intermittent red on tests that assert history semantics, for a
  reason that is not a defect in what they assert. Observed twice: in CI on
  commit `8684622` (`ctrl_p_loads_history_after_dismissing_suffix`, `last=0`)
  and locally during a full-suite run
  (`space_separated_prefix_shows_suffix_and_enter_runs_typed_bytes`, same
  helper). The CI instance is doubly informative: the two concurrent runs of
  that same commit disagreed, one passing the canonical suite while the other
  failed this test — see `M-074` for the duplicate-run contention that made it
  likelier.
- Mitigation: raised the tolerant PTY default from 1.0 s to 5.0 s for both
  `MBX_IPC_TIMEOUT` and `MBX_HISTORY_TIMEOUT`. This cannot hide a regression in
  deadline behavior, because deadline behavior is asserted in
  `tests/bash/modules.bash` and in the dedicated production-timeout case
  (`spawn_history_shell_production_timeouts`, still 0.10 s), never here.
- Instrumentation: `wait_for_count`'s failure now reports poll count and
  elapsed time, the store's files with byte sizes, the exit status, stdout and
  stderr of both `history count` and `history search recent`, and any live
  helper process matching the binary under test. The previous message —
  "count never reached 2; last=0" — could not distinguish records dropped on a
  slow exchange, a coprocess that never started, a store never written, and a
  query that itself failed. The next occurrence will say which.
- What is not established: the root cause. `last=0` means *neither* of two
  records landed, which fits a severe stall but not a marginal one, so the
  timeout may be a contributing factor rather than the whole story. This is
  recorded as `Mitigated` rather than `Fixed` because the failure could not be
  reproduced in roughly 22 deliberate attempts — the ghost suite alone 6/6 and
  4/4 under full CPU saturation, every PTY binary concurrently 5/5, the full
  canonical suite 3/3 idle and 2/2 saturated. A fix that cannot be validated
  against a reproduction is a guess; widening a budget that is documented as
  deliberately wide here, and making the next failure legible, is not.
- Prevention: a test that asserts an outcome the product is explicitly
  permitted to abandon under load must either take that permission away for
  the duration of the test or assert the abandonment instead. Decide which,
  rather than inheriting a budget from production and hoping.
- Evidence: `crates/pty/tests/common/mod.rs` (`spawn_history_shell`,
  `wait_for_count`, `store_diagnostics`); CI runs 33291533211 and 33291535294
  on commit `8684622`.

## M-076 — MBX was a complete no-op on Bash 5.0, and destroyed the user's PROMPT_COMMAND doing it

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `_mbx_install_hooks` installed its prompt chain as
  `PROMPT_COMMAND=(_mbx_capture_status "${existing[@]}" _mbx_render_prompt)`,
  assuming an array `PROMPT_COMMAND` works on every Bash the project supports.
  An array `PROMPT_COMMAND` is a **Bash 5.1** feature. Bash 5.0 treats the
  variable as an ordinary string and runs element 0 only.
- Impact: on Bash 5.0 — named as supported by `README.md` ("Bash 5.x"),
  `docs/bash-compatibility.md`, and the `HRD-001` roadmap entry — MBX did
  nothing at all. Only `_mbx_capture_status` ran each prompt;
  `_mbx_render_prompt` never did, so `PS1` was never set and the shell kept its
  stock prompt (verified: `PS1` remained `\s-\v\$ `). At the same time the
  assignment discarded any pre-existing `PROMPT_COMMAND`, so a user who had
  another framework installed lost that hook and gained nothing in exchange.
  Silent in both directions.
- Why it was invisible: every assertion about the hooks inspected the
  *variable* rather than its *effect* — `${#PROMPT_COMMAND[@]}` was 2, which
  looks correct and says nothing about whether Bash will run it. Local
  development is Bash 5.2, no CI ran 5.0 before this branch added the leg, and
  `tests/bash/smoke.bash` spawns plain `bash` for its inner shells, so running
  the suite *with* a 5.0 interpreter still exercised 5.2 inside. Correcting an
  earlier claim of mine in this same session: "all three Bash suites pass on a
  from-source Bash 5.0 build" was wrong for `smoke.bash` for exactly that
  reason — only a `bash` shim earlier in `PATH` actually tests it.
- Correction: build the chain once, then install it as an array on Bash 5.1+
  and as a `;`-joined string on 5.0, unsetting the variable first so a scalar
  assignment cannot leave stale array elements behind. The 5.1+ array form is
  kept where available because a syntax error in one entry cannot then break
  its neighbours.
- Prevention: assert the *effect*, not the installation. A hook installed into
  a representation the running interpreter ignores is indistinguishable from no
  hook at all, and only an assertion about the resulting prompt can tell them
  apart. Any version-gated language feature used in the integration layer needs
  the oldest supported release in CI before the feature is relied upon.
- Evidence: `bash/hooks.bash`; `tests/bash/smoke.bash` now asserts a rendered
  `PS1` and compares the joined `PROMPT_COMMAND` rather than an element count.
  With the fix reverted, the suite fails on Bash 5.0 with "existing
  PROMPT_COMMAND did not receive the command status" and passes with it, run
  against a from-source Bash 5.0 placed first in `PATH` so the inner shells are
  genuinely 5.0.

## M-077 — `mbx history clear` failed on lock contention a user was already waiting through

- Discovered: 2026-08-30
- Status: Fixed
- Failed assumption: `BUSY_TIMEOUT_MS` (100 ms) is applied to every SQLite
  connection the history store opens, on the assumption that one budget suits
  every caller. It does not. 100 ms is exactly right for the prompt path — MBX
  must never stall the prompt waiting on another shell's lock, and dropping a
  record is the designed degradation there. It is exactly wrong for a command
  the person at the keyboard typed and is already waiting on.
- Impact: `HistoryControl::clear` opened its connection through
  `open_connection`, which does wait out contention on the *open*, and then ran
  `DELETE FROM history` with no retry at all — only the connection's 100 ms
  `busy_timeout`. A second shell writing at that moment surfaced to the user as
  `mbx: history write: database is locked` on a command that had every reason
  to succeed. Two open shells is the ordinary case for a shell integration, not
  an edge case. Found because it failed CI on PR #53, in a diff that touched no
  history code.
- Correction: `clear` now goes through `execute_batch_with_lock_retry_until`,
  machinery this file already had and this one statement had never used, with a
  new `USER_COMMAND_BUSY_DEADLINE_MS` (2 s) budget. The hot-path
  `BUSY_TIMEOUT_MS` is untouched, so nothing about the prompt's
  never-stall guarantee changes. `delete` needs nothing — it removes files
  rather than touching SQLite — so `clear` was the only user-invoked write
  exposed.
- Prevention: a single timeout constant shared between a latency-critical hot
  path and a user-invoked command encodes one policy for two situations with
  opposite requirements. When adding a budget, name the caller it is for. The
  tell here was a retry helper sitting in the same file, used by the writer
  loop and by `open_connection`, that the one statement which actually reached
  a user had skipped.
- Evidence:   `crates/cli/src/storage.rs`
  (`clear_waits_out_a_concurrent_writer_instead_of_failing`), which holds the
  write lock from a second connection for 400 ms — past the hot-path budget,
  inside the user-command one. Against the unfixed code it reproduces CI's
  exact failure, `HistoryError { kind: Write, message: "database is locked" }`.

## M-078 — `${#var-}` is a runtime bad substitution

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: Bash default-value syntax works inside `${#…}`, as in
  `local len=${#_MBX_HIGHLIGHT_PLAIN-}`.
- Impact: `_mbx_highlight_forward` aborted with `bad substitution` on every
  Right/`C-f` press before moving the cursor, and printed the error into the
  session. Invisible because no test referenced `_mbx_highlight_forward`.
- Correction: use `${#_MBX_HIGHLIGHT_PLAIN}` (unset length is 0 without
  `nounset`). Module contract advances the plain cursor.
- Prevention: never put `-`, `=`, `+`, or `?` operators inside `${#…}`.
  Motion widgets need a contract that they actually move.
- Evidence: `bash/highlight.bash` (`_mbx_highlight_forward`);
  `tests/bash/modules.bash`.

## M-079 — Store create-then-chmod left a world-readable window

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: creating a SQLite file (or directory) then `chmod 0600`
  was equivalent to creating it with that mode. M-033 fixed widening, not
  this race.
- Impact: on a shared host, another UID could open the new database during
  the umask window. WAL/SHM were only tightened later.
- Correction: create the store file with `OpenOptionsExt` mode `0600` and
  the parent directory with `DirBuilderExt` mode `0700` before SQLite opens
  them. Socket bind still chmods immediately after `bind` (Unix sockets
  cannot be pre-created as regular files); its parent dir is created `0700`
  when missing.
- Prevention: create files and directories with the target mode; do not
  create-then-chmod. Permission tests must cover a newly created store, not
  only tightening of an existing one.
- Evidence: `ensure_restricted_store_file` / `create_store_dir` in
  `crates/cli/src/storage.rs`; `newly_created_store_file_is_owner_only`.

## M-080 — History `Drop` blocked forever on a full queue

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: ingest's non-blocking `try_send` meant Drop could
  `send(Shutdown)` and `join` the writer. `SyncSender::send` blocks when the
  queue is full.
- Impact: process exit / serve teardown with history enabled could hang if
  the writer was slow (Git enrich, lock wait) while the queue was at
  capacity.
- Correction: Drop uses `try_send(Shutdown)` and a 500 ms timed join.
  Disconnect still ends the writer after its current batch.
- Prevention: a hot-path queue that is non-blocking on ingest must also be
  non-blocking on shutdown.
- Evidence: `QueuedHistoryStore::Drop`; `drop_does_not_block_on_a_full_queue`.

## M-081 — History exclude glob was exponential in star count

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: a recursive `*` matcher on local config was cheap
  enough for every RECORD, including 64 KiB command text.
- Impact: a pathological `MBX_HISTORY_EXCLUDE` (many `*`s) could stall or
  melt ingest.
- Correction: collapse consecutive `*` atoms and bound matcher steps
  (`GLOB_STEP_BUDGET`). Over-budget patterns fail open (do not exclude)
  rather than stall.
- Prevention: untrusted haystacks plus user-supplied glob patterns need a
  work bound, not only a correctness corpus of short strings.
- Evidence: `crates/cli/src/policy.rs`;
  `glob_match_is_bounded_on_pathological_stars`.

## M-082 — Highlight accepted C0 input and skipped mid-codepoint after `\`

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: rejecting NUL and running `strip(style(x)) == x` on a
  printable corpus proved the helper was safe for arbitrary bytes. Word
  tokens copy input verbatim, so SOH/STX/CSI already in the input were
  treated as markup on strip. Quote lexing advanced one byte after `\`,
  splitting a following multibyte scalar.
- Impact: `mbx highlight` / HIGHLIGHT wire could return styled text whose
  strip was not the original; cursor maps could sit mid-glyph.
- Correction: `highlight_line` rejects every C0 byte and DEL, not only NUL.
  Escaped sequences in double quotes and backticks skip a whole UTF-8
  scalar.
- Prevention: the strip-round-trip contract must include C0/CSI in the
  input, or those bytes must be rejected at the helper boundary. Byte `+ 2`
  after `\` is not a UTF-8 advance.
- Evidence: `crates/cli/src/highlight.rs`
  (`c0_or_del_input_is_rejected`,
  `escaped_multibyte_in_quotes_does_not_split_a_scalar`).

## M-083 — Preview-row C0 glob included ESC and refused every SGR row

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: skipping the C0 range `$'\030'-$'\037'` would leave ESC
  (`$'\033'`) intact so SGR on the highlight preview row could pass the
  injection check. In Bash, `\030` is octal 24 and `\037` is octal 31; that
  range includes ESC (octal 033 / decimal 27).
- Impact: every colored preview row was refused. Module tests and a PTY
  Screen capture showed an intact prompt with no styled copy below it when
  `color=1`. A `color=0` (plain) preview still appeared, which looked like
  M-062 again.
- Correction: `_mbx_highlight_preview_row_ok` walks bytes and allows only
  code 27 among C0; SOH/STX/DEL and every other control stay refused.
- Prevention: do not encode "all C0 except ESC" as an octal glob range.
  Octal `\033` is ESC; a range that starts at `\030` includes it. Write a
  byte loop or an explicit allow-list, and assert that an SGR row is
  accepted while SOH is not.
- Evidence: `_mbx_highlight_preview_row_ok` in `bash/highlight.bash`;
  module contracts that an SGR row is allowed and SOH is refused;
  `crates/pty/tests/highlight.rs`
  (`highlight_preview_row_paints_sgr_below_an_intact_prompt`).

## M-084 — TTY row clamp split UTF-8 under a C locale

- Discovered: 2026-08-31
- Status: Fixed
- Failed assumption: `${#text}` and `${text:index:1}` walk Unicode scalars,
  so treating `code >= 128` as two columns would keep `中` intact. That is
  true only in a UTF-8 locale. In C/POSIX they walk bytes.
- Impact: the Bash-matrix CI containers (`ubuntu:20.04`/`22.04`/`24.04`)
  default to C. `_mbx_tty_clamp_row '中x' 2` copied the first byte
  (`$'\344'`) and the module suite failed on every Bash 5.x leg while the
  UTF-8 canonical suite stayed green.
- Correction: clamp with `LC_ALL=C` and consume a whole UTF-8 sequence per
  non-ASCII scalar (2/3/4 bytes from the lead byte). Signed high-byte codes
  are folded back into 0–255.
- Prevention: any Bash walk that cares about glyphs or display width must
  either force a UTF-8 locale that the host guarantees, or index bytes and
  decode UTF-8 itself. Module contracts for that walk must run under
  `LC_ALL=C`, not only the developer's UTF-8 locale.
- Evidence: `_mbx_tty_clamp_row` in `bash/engine.bash`; C-locale clamp
  contracts in `tests/bash/modules.bash`.


## M-085 — Review-close evidence was recorded for an assert that had no test

- Discovered: 2026-09-03
- Status: Fixed
- Failed assumption: the ADR 0013 review-close plan and the roadmap recorded
  "H-1–H-6, O-1–O-5, and M-1 have module/PTY evidence" once the close slices
  landed. H-4 — `MBX_HIGHLIGHT` unset installs no highlight widgets and typing
  stays stock — had no test anywhere: every highlight PTY case set
  `MBX_HIGHLIGHT=1`, and no module case covered the absent configuration.
- Impact: the roadmap and the close plan cited evidence that did not exist,
  and the opt-in feature's absent-configuration behavior — the exact class
  M-024 and M-037 already turned into a prevention rule — was unprotected. A
  regression that installed highlight widgets without `MBX_HIGHLIGHT=1` would
  have passed the entire canonical suite. Found by a read-only review that
  tried to resolve each cited evidence ID to a named test; O-1 survived the
  same check only in substance (its snapshot cap is asserted against an
  80-row fixture and its cycle bound became OV-3), never by its ID.
- Correction: added the H-4 PTY case (`highlight_unset_installs_no_widgets`,
  confirmed to fail when the `MBX_HIGHLIGHT` gate is neutered) and annotated
  the existing O-1 evidence in `tests/bash/modules.bash` with its ID and the
  pointer to OV-3, so the next ID-resolution check resolves both.
- Prevention: an evidence ID recorded as covered must resolve to a named,
  runnable test before the claim is written down — cite the test name in the
  same sentence as the claim. Every opt-in feature needs the absent,
  explicit-off, and explicit-on configurations exercised (M-024, M-037); the
  review close only added enabled-side tests, which is why the gap was
  invisible to the pass that should have caught it.
- Evidence: `crates/pty/tests/highlight.rs`
  (`highlight_unset_installs_no_widgets`); `tests/bash/modules.bash`
  (O-1 annotation above the eight-candidate snapshot cap); the resolving
  review grepped every H-1–H-6/O-1–O-5 ID against the test files and found H-4
  absent.
