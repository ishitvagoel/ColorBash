# HLT-003: hostile-input and exact-byte stripping gates

Status: `complete` (2026-08-31) for slices 1–2. p99 percentiles stay
`deferred` per `docs/latency-budget-deferral.md`. ADR 0015 resolved
`M-064`: live color paints on the reserved preview row and
`READLINE_LINE` stays plain, so this gate no longer blocks Phase 6.

## Why this plan

Phase 6 required exact-byte recovery and hostile-input safety before
`HLT-003` could move toward `complete`. Wrap/Enter/motion evidence exists
(H-1–H-6, M-1). Preview-row rendering (ADR 0015) is the live path; Enter
executes the plain buffer.

| Rank | Item | Why this order |
| --- | --- | --- |
| 1 | **Hostile corpus + strip round-trip** (this slice) | Rust and Bash strip must agree before PTY hostile gates claim safety. |
| 2 | **PTY hostile execute-plain** | One Enter after hostile printable lines runs exact plain bytes; C0 insert refused. |
| 3 | **Highlight p99 bench** | `deferred`; record only when a functional defect is proven. |
| — | macOS `HRD-001` pairwise | Needs a macOS host (ADR 0012). |

## Goal (slice 1) — `complete`

1. Shared printable hostile corpus in `crates/cli/src/highlight.rs` tests.
2. `strip_to_plain(highlight_line(row)) == row` for every row with color.
3. Cursor map at start, middle, and end without drift.
4. Lexer must advance past non-ASCII bytes (no infinite loop).
5. Bash module: real `mbx highlight` corpus strip-round-trip in plain mode.

## Goal (slice 2) — `complete`

1. PTY: hostile printable line executes plain on one Enter.
2. C0 insert refused: module test on `_mbx_highlight_self_insert` (PTY ESC is
   flaky on the harness; bell/meta path is not the self-insert contract).

## Asserts (slice 1)

| ID | Evidence |
| --- | --- |
| S-1 | Rust hostile corpus strip round-trip |
| S-2 | Rust cursor map at 0, mid, len |
| S-3 | Rust UTF-8 lexer advances |
| S-4 | Module: `mbx highlight` corpus strip round-trip |

## Asserts (slice 2)

| ID | Evidence |
| --- | --- |
| P-1 | PTY hostile printable line; one Enter; plain stdout |
| P-2 | Module: `_mbx_highlight_self_insert` refuses C0 bytes |

## Validate

```bash
cargo test -p mbx highlight::
bash tests/bash/modules.bash
cargo test -p mbx-pty --test highlight -- --nocapture
bash tests/run.bash
```

## Stop

Do not chase p99 percentiles. Do not start a second rendering strategy.
