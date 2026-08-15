#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
cd "$ROOT"

cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash -n bash/*.bash scripts/*.bash tests/bash/*.bash tests/integration/*.bash tests/run.bash
cargo build --workspace
bash tests/integration/protocol.bash "$ROOT/target/debug/mbx"
bash tests/bash/smoke.bash "$ROOT/target/debug/mbx"
