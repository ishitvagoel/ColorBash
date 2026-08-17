# Foundation UX specification

## Product promise

MBX is Bash with a clearer interaction layer. The prompt reveals useful context
without hiding the command model, changing syntax, or executing suggestions.

## Prompt hierarchy

The first line is contextual; the second line is a stable input anchor.

```text
~/projects/color-bash  git:main ~2 ?1  exit 1  2.5s
> 
```

Only relevant segments appear. Two spaces separate segment groups, while details
inside a segment use one space. The path is always present. Failure and danger
states are never conveyed by color alone.

Priority order:

1. `! PROD · host · user` when explicitly configured;
2. `ssh:host` for an SSH session when not marked production;
3. compact path, with `~` substitution and long-path elision;
4. `git:branch`, plus `+N`, `~N`, and `?N` change counts;
5. `exit N` only after failure;
6. elapsed time only at or above two seconds.

## Semantic roles

The renderer names presentation intent before choosing ANSI values:

| Role | Purpose | Plain fallback |
| --- | --- | --- |
| primary | stable input anchor | `>` |
| path | current location | path text |
| git clean | clean repository context | `git:branch` |
| git dirty | changed repository context | counts remain visible |
| warning | SSH or caution context | `ssh:` label |
| danger | explicit production state | `! PROD` label |
| error | previous non-zero status | `exit N` |
| muted | secondary duration | duration text |

`MBX_ICONS=nerd` is an explicit enhancement. Auto mode currently stays with
font-safe text because terminal capability detection cannot prove that a Nerd
Font is installed. `NO_COLOR`, redirected output, and `TERM=dumb` use plain text.

## Interaction principles

- Prompt updates must not move or execute the command buffer.
- Failure of the native renderer must produce a usable Bash prompt on the same
  prompt cycle where practical.
- Production context must be explicit configuration, never inferred from an
  unreliable host-name heuristic.
- Git metadata may describe repository state but must never select executables or
  scripts through repository data or implicit/relative lookup, or render raw
  terminal controls. Caller-supplied absolute `PATH` entries are trusted
  configuration.
- Motion is absent; terminal rendering should feel immediate and stable.
- Full-screen programs own the terminal while running. MBX writes only during the
  Bash prompt lifecycle.

## Prototype states

Normal:

```text
~/projects
>
```

Git with changes:

```text
~/projects/api  git:feature/auth +1 ~2 ?1
>
```

Previous failure:

```text
~/projects/api  git:main  exit 127
>
```

Production:

```text
! PROD · payments-api · root  /srv/app  git:main
>
```

No helper / limited terminal:

```text
~/projects/api  git:main  exit 1
>
```

## Deferred editor UX

Ghost suggestions, completion menus, type-to-filter history overlays,
highlighting, multiline guides, and command palettes remain design
requirements, not prototype claims. An opt-in history ghost (`MBX_GHOST=1` with
`MBX_HISTORY=1`, ADR 0010) can show a sidecar prefix match after the cursor;
Enter runs only the typed prefix. Right accepts the full row; Left, Home, Up,
and Down dismiss or restore history navigation; Alt-F / Ctrl-Right accept one
word; Ctrl-X Ctrl-N / Ctrl-P cycle other prefix matches. It is not dim
after-every-key paint. An explicit history-search chord (`Ctrl-X` then `h`,
ADR 0009) can insert one sidecar match and cycle a bounded snapshot;
`Ctrl-X` then `l` restores the typed line. It is not the interactive overlay
shown above. The opt-in sidecar can store and query history from the CLI. Each
later feature must insert or propose ordinary Bash text and must never
auto-execute. The Readline research and ADR 0003 define the validation needed
before dim highlighting and overlays are implemented.
