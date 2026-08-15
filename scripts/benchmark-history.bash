#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
cd "$ROOT"

ITERATIONS=${MBX_BENCH_ITERATIONS:-200}
if [[ ! $ITERATIONS =~ ^[1-9][0-9]*$ ]]; then
    printf 'benchmark: MBX_BENCH_ITERATIONS must be a positive integer\n' >&2
    exit 2
fi

# HIST-004 query budgets are 10 ms p95 on a warm 100k-row reader.
QUERY_P95_NS=10000000

output=$(
    MBX_BENCH_ITERATIONS=$ITERATIONS cargo test -p mbx --release --lib \
        corpus::tests::load_100k_and_measure_query_percentiles -- \
        --ignored --nocapture --exact
)

printf '%s\n' "$output"

fail=0
while IFS= read -r line; do
    [[ $line == area=* ]] || continue
    area=
    p95_ns=
    for field in $line; do
        case $field in
            area=*) area=${field#area=} ;;
            p95_ns=*) p95_ns=${field#p95_ns=} ;;
        esac
    done
    [[ -n $p95_ns ]] || continue
    case $area in
        history_query_recent | history_query_prefix | history_query_cwd) ;;
        *) continue ;;
    esac
    if ((p95_ns >= QUERY_P95_NS)); then
        printf 'benchmark: %s p95 %s ns exceeds budget %s ns\n' \
            "$area" "$p95_ns" "$QUERY_P95_NS" >&2
        fail=1
    fi
done <<<"$output"

if ((fail)); then
    printf 'benchmark: one or more history budgets failed\n' >&2
    exit 1
fi
