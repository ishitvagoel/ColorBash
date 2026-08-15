# Bash history admission and multiline behavior (PTY evidence)

Date: 2026-08-15. Environment: GNU Bash 5.2.21 on Linux/WSL2, driven through the
genuine PTY driver in `crates/pty` (tests in `crates/pty/tests/history_admission.rs`).

This characterization is the evidence base for ADR 0005's capture semantics
(`HIST-001`) and for the Phase 3A vertical-slice contract (`HIST-003`). It
describes how an interactive Bash session admits commands into its own history
list, because that admission is the sidecar's authority: the sidecar records
Bash-normalized text only after Bash itself has admitted the entry.

## Method

Each case spawns `bash --noprofile --norc -i` with a controlled `HISTFILE` in a
fresh temp `HOME`, types the scenario through the PTY, then runs a sourced dump
script that executes `history -a` and prints a completion marker. The test reads
the `HISTFILE` from disk after the marker, so assertions never depend on
readline echo or prompt timing. One additional case runs a noninteractive shell
without a PTY to prove no history file is created.

## Findings

### Simple commands are admitted

Every executed simple command appears in `HISTFILE` after `history -a`.

### `HISTCONTROL=ignorespace` drops leading-space commands

A command typed with a leading space is not admitted. The entry is absent from
the file while ordinary commands remain.

### `HISTCONTROL=ignoredups` keeps one consecutive duplicate

Two consecutive identical commands collapse to a single entry. A non-consecutive
repeat of the same command is admitted again (Bash compares only consecutive
entries for `ignoredups`).

### `HISTCONTROL=ignoreboth` applies both rules

Leading-space commands are dropped and consecutive duplicates collapse in the
same session.

### `HISTCONTROL=erasedups` removes earlier occurrences

Re-running a command erases all earlier occurrences; the file contains the
latest occurrence only, positioned at the later point in the list.

### `HISTIGNORE` excludes matching commands

A `HISTIGNORE='rm *'` pattern prevents the matching command from being admitted.
The ignored command still executes.

### `set +o history` suppresses admission

Commands typed while history is disabled are not admitted; the file contains
only commands typed while history was enabled. `HISTCMD` does not advance while
history is disabled; after history is enabled again, the next admitted command
resumes at the next number. The `set -o history` command itself is read while
history is disabled and is therefore not admitted.

### `history -s` injects without executing

`history -s 'text'` adds `text` as the next entry without running it. The PTY
transcript shows no output or error for the injected text, and the file contains
the injected entry plus the real command that followed.

### Multiline commands are stored folded into one entry

A backslash-continuation (`echo one \` + Enter + `two`) is admitted as the
single space-joined entry `echo one two`. A compound command spanning multiple
lines (`if true; then` / `echo if-branch` / `fi`) is admitted as the single
folded entry `if true; then echo if-branch; fi`, both in the in-memory `history`
listing and in `HISTFILE`. There is no embedded newline in the stored text.

### Deletion renumbers entries; `HISTCMD` does not go backwards

`history -d N` removes the entry and renumbers the remaining list. The
next-command counter `HISTCMD` continues monotonically past the deleted slot
(observed: after `echo a`; `echo b`; `echo c`; `history -d 2`, the next command
reports `HISTCMD=4`). It advances for admitted entries, does not advance for
history-off omissions, and is not decremented by deletion. `HISTCMD` is
therefore a monotonic sequence counter, not a stable entry identifier.

### Exit flush behavior

On `exit`, Bash writes the session list to `HISTFILE` (overwrite semantics).
With `shopt -s histappend`, the flush appends and preserves prior file entries
exactly once. A seeded-file PTY test also confirms that `history -a` appends new
entries without rewriting or duplicating prior entries. The sidecar must never
rely on any of these modes; it must only read after its own `history -a`
equivalent observation and leave the file untouched.

### Noninteractive shells do not write history

`bash --noprofile --norc -c '...'` with a `HISTFILE` set does not create the
file. Only interactive shells have an active history list.

## Implications for the sidecar contract

1. **Admission authority.** The sidecar must observe the same folding and
   filtering Bash applies. Raw readline text, `$BASH_COMMAND`, or per-keystroke
   capture would record entries Bash later omits (leading space, duplicates,
   `HISTIGNORE`, history-off) or the unfolded multiline form Bash never stores.
2. **Multiline text.** The recorded command text must be the folded single-entry
   form Bash stores; the sidecar cannot reconstruct it from separate lines.
3. **Sequence and identity.** `HISTCMD` advances only for admitted entries,
   remains ahead after deletion, and is not a stable row identifier, so
   `(session_id, event_sequence)` from the sidecar's own counter is the stable
   idempotency key, with `HISTCMD` retained only as a diagnostic number.
4. **Drop behavior.** Commands Bash drops (space, dup, pattern, history-off) are
   dropped from the sidecar too; the sidecar never fabricates an entry.
5. **Capture boundary.** Observation must occur when Bash has already folded and
   admitted the entry (the prompt boundary after command completion), not at
   readline read time.
6. **`.bash_history` invariance.** The sidecar reads `HISTFILE` only for
   comparison tests; it never writes or truncates it. The exit-flush modes above
   are Bash's own behavior and must be controlled for in the G2 same-command
   comparison rather than fought.

## Open items for later phases

- `HISTFILESIZE`/`HISTSIZE` truncation timing on large sessions.
- Renumbering interaction with `history -c` and concurrent sessions.
- Whether `PROMPT_COMMAND`-based frameworks or `HISTCONTROL` combinations
  change admission ordering under load.
- `HISTTIMEFORMAT`-style timestamped files (not currently in scope; MBX records
  its own completion timestamp).
