#!/usr/bin/env bash
# Hold the first MBX2 QUERY RESULT until a second QUERY arrives so ghost can
# type a longer prefix before the stale generation is readable (GHST-001).
# CLI `history search` exits nonzero so a coprocess desync cannot hide behind
# the sync fallback.
set -euo pipefail

real=${MBX_REAL_BIN:?MBX_REAL_BIN must point at the real mbx helper}

if [[ ${1:-} != serve ]]; then
    if [[ ${1:-} == history && ${2:-} == search ]]; then
        exit 1
    fi
    exec "$real" "$@"
fi

coproc REAL { exec "$real" serve --stdio; }
held=

cleanup() {
    if [[ -n ${REAL_PID:-} ]]; then
        kill "$REAL_PID" 2>/dev/null || true
        wait "$REAL_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

while IFS= read -r line; do
    printf '%s\n' "$line" >&"${REAL[1]}"
    IFS= read -r -u "${REAL[0]}" response
    IFS=$'\t' read -r -a fields <<<"$line"
    if [[ ${fields[2]-} == QUERY ]]; then
        if [[ -z $held ]]; then
            held=$response
            continue
        fi
        printf '%s\n' "$held"
        held=
    fi
    printf '%s\n' "$response"
done
