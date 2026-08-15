# ADR 0007: Providers return bounded data, never execute project code

Status: Proposed; implementation deferred

## Context

Git, filesystem, Python, Node, and Docker context will enrich prompts, completion,
descriptions, validation, and diagnostics. Repository contents and external tool
output are untrusted, and provider latency must not block typing.

## Decision

Define provider capabilities around detection, completion, description, prompt
segments, and diagnostics. Results are typed data with source, confidence, expiry,
and display-safe metadata. Providers run behind deadlines and caches. They may read
known metadata or invoke explicitly allowlisted tools with safe arguments; they may
not source repository files or run project-local scripts merely for discovery.

## Alternatives

- Hard-coding all logic in the renderer prevents isolation and latency control.
- Arbitrary executable plugins create an early security and lifecycle boundary.
- Eager scanning on every prompt/keypress violates the latency contract.

## Consequences

Providers become independently testable and can progressively enhance standard
Bash candidates. Capability schemas and cache invalidation add design work.

## Risks

Tool output can contain ANSI/OSC injection; filesystem scans can be unbounded;
provider disagreement may clutter UX; container/cloud commands may trigger network
or credential activity.

## Validation plan

Start with Git and filesystem providers. Fuzz output sanitization, enforce byte/time
limits, record cache hit/miss latency, and test hostile branch/file names. Add
Python/Node/Docker only after provider interfaces and cancellation behavior are
measured.

