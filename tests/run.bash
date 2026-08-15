#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
cd "$ROOT"

cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash -n bash/*.bash scripts/*.bash tests/bash/*.bash tests/bash/fixtures/*.bash \
    tests/integration/*.bash tests/run.bash
bash tests/bash/modules.bash "$ROOT/target/debug/mbx"
bash tests/integration/protocol.bash "$ROOT/target/debug/mbx"
bash tests/bash/smoke.bash "$ROOT/target/debug/mbx"
