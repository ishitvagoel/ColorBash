#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
cd "$ROOT"

output=$(
    cargo test -p mbx --release --lib \
        corpus::tests::schema_v1_100k_corpus_migrates_to_v2 -- \
        --ignored --exact --nocapture
)

printf '%s\n' "$output"
