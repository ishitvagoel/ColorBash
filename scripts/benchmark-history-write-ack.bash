#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
cd "$ROOT"

COMMANDS=${MBX_BENCH_COMMANDS:-200}
if [[ ! $COMMANDS =~ ^[1-9][0-9]*$ ]]; then
    printf 'benchmark: MBX_BENCH_COMMANDS must be a positive integer\n' >&2
    exit 2
fi

# HIST-004 write-ack budget is measured at the Bash prompt boundary.
WRITE_ACK_P95_US=2000
WRITE_ACK_P99_US=5000

cargo build --release --workspace >/dev/null

target_dir=${CARGO_TARGET_DIR:-$ROOT/target}
if [[ $target_dir != /* ]]; then
    target_dir=$ROOT/$target_dir
fi
release_bin=$target_dir/release/mbx
if [[ ! -x $release_bin ]]; then
    printf 'benchmark: release mbx missing: %s\n' "$release_bin" >&2
    exit 2
fi

output=$(
    MBX_TEST_BIN="$release_bin" \
        MBX_BENCH_COMMANDS="$COMMANDS" \
        cargo test -p mbx-pty --release --test history_write_ack \
        measure_prompt_boundary_write_ack_percentiles -- \
        --ignored --nocapture --exact
)

printf '%s\n' "$output"

fail=0
parsed=0
while IFS= read -r line; do
    [[ $line == area=history_write_ack* ]] || continue
    p95_us=
    p99_us=
    for field in $line; do
        case $field in
            p95_us=*) p95_us=${field#p95_us=} ;;
            p99_us=*) p99_us=${field#p99_us=} ;;
        esac
    done
    if [[ -z $p95_us || -z $p99_us ]]; then
        printf 'benchmark: malformed history_write_ack percentile line\n' >&2
        exit 1
    fi
    parsed=$((parsed + 1))
    if ((p95_us >= WRITE_ACK_P95_US)); then
        printf 'benchmark: history_write_ack p95 %s us exceeds budget %s us\n' \
            "$p95_us" "$WRITE_ACK_P95_US" >&2
        fail=1
    fi
    if ((p99_us >= WRITE_ACK_P99_US)); then
        printf 'benchmark: history_write_ack p99 %s us exceeds budget %s us\n' \
            "$p99_us" "$WRITE_ACK_P99_US" >&2
        fail=1
    fi
done <<<"$output"

if ((parsed != 1)); then
    printf 'benchmark: expected one history_write_ack percentile line, got %s\n' \
        "$parsed" >&2
    exit 1
fi

if ((fail)); then
    printf 'benchmark: prompt-boundary write-ack budget failed\n' >&2
    exit 1
fi
