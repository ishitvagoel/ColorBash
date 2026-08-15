#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
MBX_BENCH_BIN=${1:-"$ROOT/target/release/mbx"}
ITERATIONS=${MBX_BENCH_ITERATIONS:-500}

if [[ ! -x $MBX_BENCH_BIN ]]; then
    printf 'benchmark: executable not found: %s\n' "$MBX_BENCH_BIN" >&2
    exit 2
fi
if [[ ! $ITERATIONS =~ ^[1-9][0-9]*$ ]]; then
    printf 'benchmark: MBX_BENCH_ITERATIONS must be a positive integer\n' >&2
    exit 2
fi
command -v git >/dev/null 2>&1 || {
    printf 'benchmark: git is required for the prompt-provider workload\n' >&2
    exit 2
}

bench_tmp=$(mktemp -d "${TMPDIR:-/tmp}/mbx-prompt-benchmark.XXXXXXXX")
coproc_pid=
coproc_in=
coproc_out=

cleanup() {
    if [[ -n ${coproc_in:-} ]]; then
        exec {coproc_in}>&- 2>/dev/null || true
    fi
    if [[ -n ${coproc_out:-} ]]; then
        exec {coproc_out}<&- 2>/dev/null || true
    fi
    if [[ -n ${coproc_pid:-} ]] && kill -0 "$coproc_pid" 2>/dev/null; then
        kill "$coproc_pid" 2>/dev/null || true
        wait "$coproc_pid" 2>/dev/null || true
    fi
    rm -rf -- "$bench_tmp"
}
trap cleanup EXIT INT TERM

git -c init.defaultBranch=main init -q "$bench_tmp/repository"

now_us() {
    local seconds=${EPOCHREALTIME%.*}
    local fraction=${EPOCHREALTIME#*.}000000
    fraction=${fraction:0:6}
    REPLY=$((10#$seconds * 1000000 + 10#$fraction))
}

request_prompt() {
    local request_id=$1
    local response

    (trap '' PIPE; printf 'MBX1\t%s\tPROMPT\t%s\t0\t-\t3\n' \
        "$request_id" "$bench_tmp/repository" >&"$coproc_in")
    IFS= read -r -t 1 response <&"$coproc_out"
    [[ $response == "MBX1"$'\t'"$request_id"$'\t'PROMPT$'\t'* ]]
}

percentile() {
    local percentile=$1
    local count=${#sorted_samples[@]}
    local rank=$(((count * percentile + 99) / 100 - 1))
    ((rank >= 0)) || rank=0
    REPLY=${sorted_samples[rank]}
}

coproc MBX_PROMPT_BENCH_COPROC { exec "$MBX_BENCH_BIN" serve --stdio; }
coproc_pid=$MBX_PROMPT_BENCH_COPROC_PID
exec {coproc_out}<&"${MBX_PROMPT_BENCH_COPROC[0]}"
exec {coproc_in}>&"${MBX_PROMPT_BENCH_COPROC[1]}"
original_out=${MBX_PROMPT_BENCH_COPROC[0]}
original_in=${MBX_PROMPT_BENCH_COPROC[1]}
exec {original_out}<&-
exec {original_in}>&-
unset MBX_PROMPT_BENCH_COPROC MBX_PROMPT_BENCH_COPROC_PID

# Populate the repository-status cache before recording warm-path samples.
request_prompt 0

samples=()
for ((iteration = 1; iteration <= ITERATIONS; iteration++)); do
    now_us
    started=$REPLY
    request_prompt "$iteration"
    now_us
    samples+=("$((REPLY - started))")
done

mapfile -t sorted_samples < <(printf '%s\n' "${samples[@]}" | sort -n)
percentile 50
p50=$REPLY
percentile 95
p95=$REPLY
percentile 99
p99=$REPLY

printf 'workload=warm-prompt-git iterations=%s p50_us=%s p95_us=%s p99_us=%s\n' \
    "$ITERATIONS" "$p50" "$p95" "$p99"
