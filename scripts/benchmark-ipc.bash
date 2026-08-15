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

now_ns() {
    date +%s%N
}

mean_ns() {
    local total=$1
    printf '%s' "$((total / ITERATIONS))"
}

for ((i = 0; i < 10; i++)); do
    "$MBX_BENCH_BIN" handshake >/dev/null
done

started=$(now_ns)
for ((i = 0; i < ITERATIONS; i++)); do
    "$MBX_BENCH_BIN" handshake >/dev/null
done
finished=$(now_ns)
per_call_total=$((finished - started))
printf 'transport=process-per-call iterations=%s total_ns=%s mean_ns=%s\n' \
    "$ITERATIONS" "$per_call_total" "$(mean_ns "$per_call_total")"

coproc MBX_BENCH_COPROC { exec "$MBX_BENCH_BIN" serve --stdio; }
coproc_pid=$MBX_BENCH_COPROC_PID
exec {coproc_out}<&"${MBX_BENCH_COPROC[0]}"
exec {coproc_in}>&"${MBX_BENCH_COPROC[1]}"
original_out=${MBX_BENCH_COPROC[0]}
original_in=${MBX_BENCH_COPROC[1]}
exec {original_out}<&-
exec {original_in}>&-

(trap '' PIPE; printf 'MBX1\t0\tPING\n' >&"$coproc_in")
IFS=$'\t' read -r magic request_id kind <&"$coproc_out"
[[ $magic == MBX1 && $request_id == 0 && $kind == PONG ]]

started=$(now_ns)
for ((i = 1; i <= ITERATIONS; i++)); do
    (trap '' PIPE; printf 'MBX1\t%s\tPING\n' "$i" >&"$coproc_in")
    IFS=$'\t' read -r magic request_id kind <&"$coproc_out"
    [[ $magic == MBX1 && $request_id == "$i" && $kind == PONG ]]
done
finished=$(now_ns)
coproc_total=$((finished - started))
printf 'transport=bash-coprocess iterations=%s total_ns=%s mean_ns=%s\n' \
    "$ITERATIONS" "$coproc_total" "$(mean_ns "$coproc_total")"
exec {coproc_in}>&-
exec {coproc_out}<&-
wait "$coproc_pid"

bench_tmp=$(mktemp -d "${TMPDIR:-/tmp}/mbx-benchmark.XXXXXXXX")
socket_path=$bench_tmp/engine.sock
socket_pid=
cleanup() {
    if [[ -n $socket_pid ]] && kill -0 "$socket_pid" 2>/dev/null; then
        kill "$socket_pid" 2>/dev/null || true
        wait "$socket_pid" 2>/dev/null || true
    fi
    [[ ! -S $socket_path ]] || unlink "$socket_path"
    rmdir "$bench_tmp" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

"$MBX_BENCH_BIN" serve --socket "$socket_path" &
socket_pid=$!
for ((i = 0; i < 100; i++)); do
    [[ -S $socket_path ]] && break
    sleep 0.01
done
[[ -S $socket_path ]] || {
    printf 'benchmark: Unix socket server did not become ready\n' >&2
    exit 1
}
"$MBX_BENCH_BIN" benchmark-client --socket "$socket_path" --iterations "$ITERATIONS"
