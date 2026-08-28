#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
cd "$ROOT"

command -v cargo >/dev/null 2>&1 || {
    printf 'mbx setup: Rust/Cargo is required (https://rustup.rs).\n' >&2
    exit 2
}

cargo build --release --workspace
printf '\nBuilt %s/target/release/mbx\n' "$ROOT"
printf 'Try it in the current Bash session with:\n\n'
printf '  source %q\n\n' "$ROOT/bash/init.bash"
printf 'This script does not modify ~/.bashrc.\n'
printf 'For a persistent install with a feature profile:\n\n'
printf '  bash %q --bashrc\n' "$ROOT/scripts/install.bash"
printf 'To pick options interactively:\n\n'
printf '  bash %q --interactive\n' "$ROOT/scripts/install.bash"

