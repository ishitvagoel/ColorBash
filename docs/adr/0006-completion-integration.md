# ADR 0006: Adapt existing Bash completion before enriching it

Status: Proposed; experiment required

## Context

Bash completion definitions encode quoting, filenames, spacing, command-specific
logic, and dynamic shell state. MBX needs structured candidates but cannot reduce
compatibility to a new provider-only ecosystem.

## Decision

Stock programmable completion remains authoritative. Build an adapter that invokes
supported existing specs with the expected `COMP_*` environment, captures
`COMPREPLY` and `compopt` behavior, and normalizes candidates without changing
their insertion value. Provider descriptions/kinds are additive. Unknown or failed
specs fall through to stock completion.

## Alternatives

- Replacing Bash completion would lose broad command coverage and quoting rules.
- Parsing `complete -p` text alone misses function side effects.
- Provider-only completion is easier but is not a compatible Bash enhancement.

## Consequences

Users retain their current definitions while MBX can rank and describe results.
The adapter must represent both candidate display metadata and exact insertion
semantics.

## Risks

Completion functions can execute arbitrary user-configured code, mutate globals,
call `compopt`, or be slow. Running them asynchronously may observe different shell
state. Quoting edge cases can corrupt the buffer.

## Validation plan

Prototype file completion and one `-F` command completion. Snapshot all `COMP_*`
inputs, candidate bytes, spacing, and quoting against stock Bash. Test aliases,
redirections, spaces, Unicode, incomplete quotes, `--`, and nested subcommands
before building a popup.

