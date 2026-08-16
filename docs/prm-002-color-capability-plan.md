# PRM-002 slice: color capability (T-1–T-6)

Status: `complete` for T-1–T-6 (2026-08-16). Do not mark `PRM-002`, `G0`, `G2`,
or `HIST-007` complete. PTY wrap-column probes remain `discovery`. Remaining `G2`
is still foreign-user open and the write-ack budget.

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Color capability negotiation (this plan) | Redirected-output and display-width helpers are recorded. This host can prove TERM/COLORTERM → additive flags → 16 / 256 / truecolor SGR with unit and protocol tests. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. Do not fake `seteuid`. |
| 3 | Write-ack p95/p99 budget miss | Correctness recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite. |
| — | PRM-002 wrap-column PTY probes / PRM-004 / EDT-001 | Wrap math and representative percentiles stay blocked or discovery. |
| — | HRD-001 macOS PTY matrix | Needs a macOS host. |

## Goal

1. Color-enabled prompts select 16-color, 256-color, or truecolor SGR from additive
   flags, not a single hard-coded `38;5` palette.
2. `FLAG_NO_COLOR` still wins. Piped defaults still insert `FLAG_NO_COLOR`.
3. Coprocess, per-call, and fallback carry the same raw flag integer.
4. `PRM-002` stays `discovery` (wrap-column PTY probes remaining). `G0` stays
   `validation`. Remaining `G2` stays foreign-user open and write-ack budget.

## Out of scope (hard)

- PTY wrap-column / cursor-position probes (`\e[6n`, DSR)
- Readline redisplay or two-line prompt changes
- Retargeting piped-default `FLAG_NO_COLOR` (`M-009`) or `--flags` under a pipe (`M-011`)
- Foreign-user open, write-ack product optimization, history storage
- Marking `PRM-002`, `G0`, `G2`, or `HIST-007` complete
- FND-001 CI SHA refresh
- Committing, pushing, or editing shell startup files unless asked

## Additive prompt flags

| Bit | Value | Meaning |
| ---: | ---: | --- |
| 6 | 64 | prefer 16-color ANSI SGR |
| 7 | 128 | prefer truecolor (`38;2`) SGR |

When `FLAG_NO_COLOR` (bit 0) is set, ignore bits 6 and 7. Else `FLAG_TRUECOLOR`
wins over `FLAG_COLOR_16`. If neither extra bit is set and color is enabled, keep
today's 256-color `38;5` sequences (backward compatible).

Environment policy (`prompt_flags` / `_mbx_prompt_flags`):

- `color_disabled` unchanged (`NO_COLOR`, `MBX_COLOR=never`, `TERM=dumb`, `!tty`).
- else if `COLORTERM` is `truecolor` or `24bit` (case-insensitive): `FLAG_TRUECOLOR`.
- else if `TERM` contains `256color` or is `xterm-direct`: no extra color bits (256).
- else: `FLAG_COLOR_16`.

Do not infer color from `stdout.is_terminal()` after `--flags` (`M-011`).

## Truecolor RGB triples (approximating the 256 palette)

| Role | RGB |
| --- | --- |
| path | 135, 215, 255 |
| primary | 135, 175, 215 |
| repository clean | 135, 215, 135 |
| repository dirty / warning | 255, 215, 135 |
| danger | 255, 0, 0 |
| error | 255, 135, 135 |
| muted | 138, 138, 138 |

## 16-color SGR mapping

| Role | SGR |
| --- | --- |
| path / primary | `1;36` |
| repository clean | `1;32` |
| repository dirty / warning | `1;33` |
| danger / error | `1;31` |
| muted | `1;30` |

## Cases

| Case | What | Pass |
| --- | --- | --- |
| T-1 | Piped / `FLAG_NO_COLOR` | Plain text, no CSI; `tests/integration/protocol.bash` piped prompt case |
| T-2 | TTY `TERM=xterm-256color`, no extra bits | Native prompt contains `38;5;`, not `38;2;` or bare `1;36` path SGR |
| T-3 | `COLORTERM=truecolor` / `FLAG_TRUECOLOR` | Native prompt contains `38;2;`, not `38;5;` |
| T-4 | `TERM=xterm` / `FLAG_COLOR_16` | Native prompt contains `1;36`, not `38;5;` or `38;2;` |
| T-5 | `--flags 34` under a pipe | Color preserved (`M-011`); unknown bit `1<<8` or higher preserved (`M-017`) |
| T-6 | Fallback with T-2/T-3/T-4 flags | Path segment uses the same SGR family (`38;5` / `38;2` / `1;36`) |

## Implementation notes

- Flag constants and accessors: `crates/protocol/src/lib.rs`, `bash/protocol.bash`.
- Environment policy: `crates/cli/src/environment.rs`, `bash/config.bash`.
- Native renderer: `crates/cli/src/prompt.rs` (`color_depth`, `role_sgr`, `styled`).
- Fallback renderer: `bash/fallback.bash` (`_mbx_role_sgr`).

## Validation (recorded on Linux/WSL)

```bash
cargo test -p mbx --lib environment -- prompt -- cli -- --nocapture
bash tests/bash/modules.bash target/debug/mbx
bash tests/integration/protocol.bash target/debug/mbx
cargo clippy --workspace --all-targets -- -D warnings
bash tests/run.bash
```

## Follow-on

- PRM-002 wrap-column PTY probes from `RSH-004` baseline
- PRM-004 representative/dirty/cold/fallback/platform percentiles
- Remaining `G2`: foreign-user open and write-ack budget
