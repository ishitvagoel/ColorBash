# HLT-001: tolerant Bash highlight lexer

Status: `complete` (2026-08-31). ADR 0013 authorizes Strategy A highlighting;
ADR 0015 keeps `READLINE_LINE` plain and paints the helper's styled copy on
the preview row. Lexer unit tests exist; `HLT-002` wrap close is
`docs/hlt-comp-review-close-plan.md`.

## Goal

Rust lexer in `crates/cli/src/highlight.rs` classifies comments, quotes,
variables, operators, keywords, numbers, words, and whitespace with incomplete
input tolerance. `mbx highlight TEXT [--point N]` returns styled line + styled
cursor index using Readline `\001`/`\002` markers (CLI / STYLED payload). The
interactive preview strips those markers before the tty write.

## Validate

```bash
cargo test -p mbx highlight::
```
