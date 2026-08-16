# MBX agent instructions

These instructions apply to the entire repository. Keep this file concise and
put detailed design, status, and historical information in the canonical files
linked below.

Cursor also loads `.cursor/rules/*.mdc`. Those files must not contradict this
file. This file remains the complete always-on contract for every agent.
Always-apply Cursor rules are only session start and Composer handoff; other
`.mdc` files attach by glob or relevance so they do not duplicate this file on
every turn.

## Mandatory session start

Before planning, editing, or running a mutating command:

1. Read `MISTAKES.md` in full. Apply every relevant prevention rule and do not
   repeat a recorded mistake.
2. Read `docs/roadmap.md`. It is the canonical source for delivery status,
   dependencies, gates, and immediate next work.
3. Read the relevant architecture, compatibility, protocol, research, and ADR
   documents for the area being changed.
4. Inspect `git status --short` and preserve all pre-existing user and agent
   changes. Never discard or rewrite unrelated work.

If a request is for review or planning only, do not implement feature code.

## Source-of-truth map

- Product intent: `CODEX_MODERN_BASH_ARCHITECTURE.md`
- Current plan and status: `docs/roadmap.md`
- Implemented architecture and decisions: `docs/architecture.md`, `docs/adr/`
- Bash, protocol, and UX contracts: `docs/bash-compatibility.md`,
  `docs/protocol.md`, `docs/protocol-mbx2.md`, `docs/ux-spec.md`
- Investigation evidence: `docs/research/`, `docs/benchmarks/`, and tests
- Prior mistakes and prevention: `MISTAKES.md`

Follow the reconciliation rules in `docs/roadmap.md` when sources disagree. When
edits are authorized, correct stale documentation in the same change; in a
read-only task, report the discrepancy without modifying files.

## Composer implementation handoff

When the user asks for instructions, a TODO list, or a packet for Composer
(including Composer 2.5) to implement:

1. Rank remaining work from `docs/roadmap.md`. Choose **one** Composer-sized
   slice. Do not bundle blocked, host-impossible, or explicitly deprioritized
   leftovers into the same packet.
2. Deliver the TODO list as **one copyable XML document** as the primary
   artifact. Markdown bullets may introduce the packet; they must not replace it.
3. The XML must contain enough guidance that Composer can execute without
   inventing cases: bootstrap order, hard out-of-scope rules, implement items,
   docs to update, validate commands, and an explicit stop condition.
4. The XML **must** tell Composer to review its own changes after execution and
   fix every issue found before handing off. Do not mark the slice done on first
   implementation pass.
5. Do not commit, push, or edit shell startup files unless the user asked.

Use this shape. Adapt element names only when the slice needs them; keep the
contract (`hard_rules`, `bootstrap`, `implement`, `review`, `validate`, `stop`):

```xml
<composer_packet model="composer-2.5">
  <ranking why="do not pick a leftover that cannot produce evidence on this host">
    <item rank="1" status="implement_now" id="ROADMAP-ID">One slice. Why this, not the others.</item>
    <item rank="2" status="blocked_or_later" id="OTHER">Why not now.</item>
  </ranking>
  <composer_task id="slice-id">
    <follow>Plan or spec path. Do not invent extra cases.</follow>
    <hard_rules>
      Do not mark a gate or deliverable complete unless the user and the
      roadmap exit criteria both say so. Do not commit unless asked.
    </hard_rules>
    <bootstrap order="required">
      <step>Read MISTAKES.md in full.</step>
      <step>Read docs/roadmap.md and the slice plan completely.</step>
      <step>Read the code and ADRs named in the plan.</step>
      <step>git status --short. Preserve unrelated work.</step>
    </bootstrap>
    <goal>Measurable exit criteria for this slice only.</goal>
    <implement>
      <item>Concrete change with files, names, and asserts.</item>
    </implement>
    <docs>
      <file>docs/roadmap.md</file>
    </docs>
    <validate>
      <cmd>Focused cargo/bash commands first, then bash tests/run.bash</cmd>
    </validate>
    <review required="true">
      After implementation, re-read the plan and the diff. Fix every defect,
      missed assert, stale doc, or MISTAKES.md gap before stopping. Do not
      start the next ranked leftover.
    </review>
    <stop>Do not start the next slice. Do not commit unless asked.</stop>
  </composer_task>
</composer_packet>
```

When the user then asks Composer to follow that packet: execute only that
slice, then perform the `<review>` pass and fix issues before claiming done.

## Product and architecture invariants

- MBX enhances interactive Bash; it is not a shell replacement. Bash remains the
  only parser, expansion engine, executor, and job controller.
- Preserve ordinary Bash behavior: exit status, hooks, jobs, history, completion,
  aliases, functions, traps, shell options, quoting, and command bytes.
- Suggestions and selections may insert ordinary Bash text but must never execute
  it automatically.
- Helper startup, timeout, malformed output, or failure must degrade to a usable
  prompt without disabling the shell.
- Keep sourced Bash modules small and side-effect-conscious. Do not add
  `set -euo pipefail` to a file sourced into a user's interactive shell.
- `bash/prompt.bash` is the only prompt-path writer of `PS1`. Bash transport and
  fallback adapters return candidates through `REPLY`. All render paths must keep
  safety-critical semantic parity, especially production and SSH context.
- Keep protocol, configuration, lifecycle, transport, application, rendering,
  provider, and persistence responsibilities separated. Depend on the existing
  narrow ports; do not collapse them back into composition roots.
- Preserve MBX1 framing and unknown prompt-flag bits. A change to framing,
  compatibility, trust, cancellation, or multiplexing requires an ADR and may
  require a new protocol version.
- Treat paths, environment display values, history text, repository metadata,
  protocol fields, and provider output as untrusted. Bound and sanitize data
  before it can reach `PS1` or the terminal. Any narrower executable-selection
  trust boundary must be explicit in an ADR; ADR 0007 currently trusts only
  caller-supplied absolute `PATH` entries for locating Git.
- Never source repository code or select an executable through repository data,
  an empty `PATH` entry, or a relative `PATH` entry merely to discover context.
  Caller-supplied absolute `PATH` entries are the explicit trust boundary in ADR
  0007. Construct subprocesses from fixed arguments and apply end-to-end deadlines
  and acquisition bounds.
- Avoid synchronous or unbounded work in prompt and per-keystroke paths. Severe
  interactive latency is a correctness defect.
- Readline remains responsible for editing and redisplay until an accepted ADR
  and PTY evidence justify a different strategy.
- Do not overwrite or attempt to compose an unknown DEBUG trap. Duration timing
  remains opt-in until an accepted adapter changes that policy.
- The planned history sidecar must be opt-in, local-only, privacy-preserving, and
  independent of `.bash_history`. Never log command text.
- Do not add a dependency or heavyweight framework without measured need. Record
  meaningful changes to protocol, privacy, persistence, editor ownership, or
  provider execution in an ADR.

## SOLID design philosophy

Use SOLID principles as the guiding philosophy for every design, implementation,
and refactor:

- **Single responsibility:** give each module, type, and function one coherent
  reason to change; keep composition roots limited to wiring.
- **Open/closed:** add capabilities through composition and the existing extension
  seams where practical instead of repeatedly editing central dispatch/policy.
- **Liskov substitution:** every implementation of a port must preserve its full
  behavioral, failure, safety, and fallback contract.
- **Interface segregation:** define narrow interfaces around consumer needs. Do
  not create a trait for every function or force consumers to depend on methods
  they do not use.
- **Dependency inversion:** application and domain policy depend on ports and value
  types; environment, process, filesystem, terminal, transport, clock, and storage
  details belong in adapters selected at composition boundaries.

Treat SOLID as a decision framework, not a reason to overengineer. Prefer a simple
concrete value or function when there is no real substitution/change axis. Every
new abstraction must improve an identified boundary and be exercised by a
production implementation plus a focused substitute or contract test.

When tradeoffs are unavoidable, use this order:

```text
correctness > Bash compatibility > latency > UX polish > feature breadth
```

Severe latency regressions in an interactive path count as correctness failures.

## Work and roadmap discipline

- Implement the smallest coherent vertical slice. Avoid speculative broad
  rewrites and premature work on gated phases.
- Use stable roadmap IDs when work maps to a roadmap deliverable or gate.
- Update `docs/roadmap.md` in the same change that alters scope, dependencies,
  status, or completion evidence.
- Mark an item `complete` only when its exit criteria have durable evidence. Code
  existence alone does not justify `complete`; use the roadmap's status rules.
- Never weaken a gate because downstream work has already begun. Move removed
  work to `deferred` or `superseded`; do not silently delete it.
- Keep the roadmap's "Immediate next work" list short and executable.
- Do not commit, push, publish, install globally, or modify a user's shell startup
  files unless the user explicitly asks.

## Mistake-log discipline

- Before handoff, check whether your work introduced or exposed a confirmed
  mistake. When edits are authorized, add it or update its status/evidence in
  `MISTAKES.md`; during read-only work, report the needed entry instead.
- Follow the maintenance contract inside `MISTAKES.md`. Never erase fixed history,
  duplicate a cause, record speculation/backlog, or include sensitive data.
- In parallel work, designate one writer for `MISTAKES.md`.

## Validation

Run focused checks while developing, then run the canonical suite before handing
off code changes:

```bash
bash tests/run.bash
```

That suite checks Rust formatting, workspace tests, Clippy with warnings denied,
Bash syntax, module contracts, protocol integration, and the Bash compatibility
corpus. A piped interactive Bash process is not PTY evidence; terminal interaction
claims require the real PTY harness planned in `docs/roadmap.md`.

For latency-sensitive changes, build and measure release mode:

```bash
cargo build --release --workspace
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-prompt.bash target/release/mbx
MBX_BENCH_ITERATIONS=1000 bash scripts/benchmark-ipc.bash target/release/mbx
```

Record the environment and p50/p95/p99 where the relevant gate requires them.
Add focused regression tests for every bug fix and cross-language contract change.
