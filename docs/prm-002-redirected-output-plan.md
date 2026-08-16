# PRM-002 slice: redirected-output color policy (M-009)

Status: `complete` for R-1–R-4 (2026-08-16). Do not mark `PRM-002`, `G0`, `G2`,
or `HIST-007` complete. Display-width / East Asian column math remains
`discovery`. Remaining `G2` is still foreign-user open and the write-ack budget.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Redirected-output color for direct `mbx prompt` (this plan) | Open M-009. UX spec requires plain redirected output. Direct CLI ignored stdout TTY. This host can prove pipe vs `--flags` vs env without product latency work. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. Do not fake `seteuid`. |
| 3 | Write-ack p95/p99 budget miss | Correctness recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite. |
| — | PRM-002 width model / PRM-004 / EDT-001 | Width math and representative percentiles stay blocked or discovery. |
| — | HRD-001 macOS PTY matrix | Needs a macOS host. |

## Composer bootstrap (do this first, in order)

1. Read `MISTAKES.md` in full. Apply `M-009`, `M-011`, and `M-014`.
2. Read this file completely. Do not invent extra cases.
3. Read `docs/ux-spec.md` (redirected output is plain).
4. Read `crates/cli/src/environment.rs` and `parse_prompt --flags` in
   `crates/cli/src/cli.rs`.
5. Read `tests/bash/modules.bash` command-substitution color case.
6. `git status --short`. Do not discard unrelated work.
7. Implement. Do not commit unless asked.

## Goal

1. Direct `mbx prompt` with stdout not a TTY emits plain text (no CSI) even when
   env does not disable color.
2. `--flags` still replaces defaults; per-call command substitution with color
   requested still contains ANSI (`M-011`).
3. `NO_COLOR`, `MBX_COLOR=never`, and `TERM=dumb` still disable color on a TTY.
4. Handshake/help/version do not resolve prompt defaults (`M-010`).
5. `PRM-002` stays `discovery` (width model remaining). `G0` stays `validation`.

## Out of scope (hard)

- Display-width / East Asian width / wrap-math model
- 16/256/truecolor capability negotiation
- Foreign-user open, write-ack product optimization, history storage
- Marking `PRM-002`, `G0`, `G2`, or `HIST-007` complete
- Committing, pushing, or editing shell startup files unless asked

## Cases

| Case | What | Pass |
| --- | --- | --- |
| R-1 | Piped direct CLI is plain | `color_disabled(false)` inserts `FLAG_NO_COLOR` at capture; piped `mbx prompt` has no CSI |
| R-2 | `--flags` owns color under a pipe | Raw `--flags` replaces piped defaults; command substitution color test stays green |
| R-3 | Env disables on a TTY | `NO_COLOR`, `MBX_COLOR=never`, `TERM=dumb` still insert `FLAG_NO_COLOR` when stdout is a TTY |
| R-4 | Non-prompt commands skip defaults | Handshake/help/version do not call `prompt_defaults` |

## Validation (recorded on Linux/WSL)

```bash
cargo test -p mbx --lib environment -- cli
bash tests/bash/modules.bash
bash tests/run.bash
```

## Follow-on

- PRM-002 width model from `RSH-004` baseline
- PRM-004 representative/dirty/cold/fallback/platform percentiles
- Remaining `G2`: foreign-user open and write-ack budget
