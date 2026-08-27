# HLT-001: tolerant Bash highlight lexer

Status: `validation` (2026-08-27). ADR 0013 authorizes Strategy A highlighting.
Lexer unit tests exist; `HLT-002` wrap close is
`docs/hlt-comp-review-close-plan.md`. Do **not** mark `HLT-001` complete until
H-1–H-6 prove the helper is on the editing path.

## Goal

Rust lexer in `crates/cli/src/highlight.rs` classifies comments, quotes,
variables, operators, keywords, numbers, words, and whitespace with incomplete
input tolerance. `mbx highlight TEXT [--point N]` returns styled line + styled
cursor index using Readline `\001`/`\002` markers.

## Validate

```bash
cargo test -p mbx highlight::
```
