# ADR 0007: Providers return bounded data and reject implicit project execution

Status: Accepted for the bounded Git prompt provider; broader model deferred

## Context

Git, filesystem, Python, Node, and Docker context could enrich prompts,
completion, descriptions, validation, and diagnostics. Repository contents and
external tool output are untrusted, and slow discovery must not eventually block
interactive editing.

The foundation refactor needs an extension seam now for Git prompt state, but the
repository does not yet implement completion or per-keystroke features. Defining
their full capability and lifecycle model before those use cases are measured
would be speculative.

## Decision

Implement the narrow provider boundary required by the current prompt:

- `RepositoryStatusProvider` accepts a working directory and returns typed
  `RepositoryStatus` data, provider failure, or absence.
- `RepositorySegment` depends on that interface. The Rust composition root
  injects `GitRepositoryStatusProvider`; tests can inject an in-memory provider.
- the adapter resolves Git once from executable files in absolute `PATH` entries,
  stores the absolute program path, ignores empty/relative entries, and never
  falls back to a bare command name;
- a fixed `rev-parse --is-inside-work-tree` preflight distinguishes ordinary
  absence from the fixed `git status` operation. Both share a maximum 50-ms
  refresh budget, with color and filesystem monitoring disabled,
  `GIT_OPTIONAL_LOCKS=0`, `GIT_TERMINAL_PROMPT=0`, and `LC_ALL=C`;
- stdout is acquired through a 1-MiB-plus-one capped reader. Timeout, oversize,
  spawn/acquisition/wait failures, invalid UTF-8, malformed output, and a status
  command failure are typed outcomes. Returned text is sanitized centrally by
  the renderer before PS1 use;
- a 128-entry, one-second TTL cache stores the complete result (`Some`, `None`, or
  typed `Err`) and supports deterministic expiry and explicit invalidation;
- prompt degradation omits only the repository segment; diagnostics expose only
  the typed error kind and never command text; and
- providers return data and never source repository files or select executables
  from repository data, empty `PATH` entries, or relative `PATH` entries. Absolute
  `PATH` directories are explicit caller-trusted configuration.

Defer the broader provider model until completion or another consumer establishes
its requirements. The deferred work includes:

- generic detection, completion, description, prompt-segment, and diagnostic
  capabilities;
- common source, confidence, expiry, and display-safety metadata across provider
  families;
- cross-provider scheduling, cancellation, refresh, and invalidation policy;
- filesystem, Python, Node, Docker, or executable third-party providers.

## Alternatives

- Hard-coding Git process execution in the renderer would couple presentation to
  external I/O and make tests require a repository.
- Building the entire generic capability model now would commit to unmeasured
  completion and editing requirements.
- Arbitrary executable plugins create an early security and lifecycle boundary.
- Eager scanning on every prompt or keypress violates the latency contract; the
  bounded cache/refresh policy keeps that concern inside the Git adapter.

## Consequences

The implemented repository provider and prompt-segment interfaces give the
foundation a substitutable test seam without promising a general plugin API.
Provider failure degrades by omitting the Git segment. Git parsing, process
execution, prompt composition, and final sanitization remain separate concerns.

The cache keeps warm prompt requests process-free; refresh remains synchronous but
is bounded by the provider budget. The controlled warm-Git prompt workload
measured p50/p95/p99 of 718/974/1,383 us.

## Risks

Tool output can contain ANSI/OSC or PS1 injection; filesystem scans can be slow;
provider disagreement may clutter future UX; container or cloud commands may
trigger network or credential activity. Central sanitization and the Git-specific
limits address the implemented prompt slice.

On the normal timeout path, the process runner attempts and tests direct-child
kill/reap; a failed state check, kill, or wait becomes a typed cleanup error. It
does not guarantee portable process-tree termination. A descendant unexpectedly
inheriting stdout can outlive the provider, but the detached capped reader cannot
extend prompt return. Absolute `PATH` entries are trusted caller configuration. A
nonzero preflight maps to absence, so a rare fatal discovery error is
indistinguishable from a non-repository without acquiring stderr. Kernel stalls
inside `spawn`, `kill`, or `wait` are not independently cancellable. These limits
must be revisited before admitting a provider that intentionally launches
descendants or untrusted executables.

## Validation plan

The implemented slice has focused tests for pure porcelain parsing, absolute
executable resolution, fixed command construction, hostile filesystem-monitor
configuration, capped/timeout/failure acquisition, direct-child cleanup with an
inherited descendant pipe, provider substitution/degradation, and deterministic
`Some`/`None`/`Err` cache hit/expiry/invalidation/capacity behavior. Release
evidence is recorded in `docs/benchmarks/2026-08-15-solid-hardening.md`.

Before adding a second provider family, define its consumer-specific lifecycle,
process-tree, metadata, and caching needs rather than generalizing this Git port.
Add Python, Node, or Docker only after a real consumer supports the required
security and cancellation behavior.
