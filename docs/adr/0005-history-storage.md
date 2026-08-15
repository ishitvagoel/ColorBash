# ADR 0005: Preserve Bash history and add a local sidecar later

Status: Proposed; implementation deferred

## Context

Search and ranking need cwd, repository, time, duration, status, host, and session
metadata. `.bash_history` remains the compatibility source and may encode user
privacy choices through `HISTCONTROL` and related settings.

## Decision

Do not replace or rewrite Bash history. In the history phase, add an optional local
sidecar database owned by the user, with SQLite as the leading candidate. Capture
only at the prompt lifecycle boundary, respect commands omitted by Bash history
policy where observable, provide exclusions, and disable all command-text
telemetry.

## Alternatives

- Extending the flat history file cannot represent/query metadata safely.
- Replacing Bash history breaks tooling and shell behavior.
- Remote sync is outside MVP privacy and reliability boundaries.

## Consequences

Enhanced search can be disabled or deleted independently. Two stores require
deduplication and clear retention rules.

## Risks

Commands may contain secrets; concurrent shells can race; crash timing can mismatch
status and command; database growth and migrations need bounds.

## Validation plan

Write a privacy threat model and schema ADR update, test 100k+ rows and concurrent
sessions, verify `HISTCONTROL` cases, redact/exclude known secret forms, and prove
that disabling/removing the sidecar leaves Bash history unchanged.

