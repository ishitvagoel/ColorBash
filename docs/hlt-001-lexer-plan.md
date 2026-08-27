# HLT-001: tolerant Bash highlight lexer

Status: `complete` (2026-08-27). ADR 0013 authorizes Strategy A highlighting.

## Goal

Rust lexer in `crates/cli/src/highlight.rs` classifies comments, quotes,
variables, operators, keywords, numbers, words, and whitespace with incomplete
input tolerance. `mbx highlight TEXT [--point N]` returns styled line + styled
cursor index using Readline `\001`/`\002` markers.

## Validate

```bash
cargo test -p mbx highlight::
```
