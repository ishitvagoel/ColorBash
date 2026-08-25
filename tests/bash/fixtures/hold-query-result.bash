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

held_id=
held_gen=
held_text=

result_for() {
    local id=$1
    local gen=$2
    local text=$3
    local cmd
    case $text in
        pw*) cmd='pwd # MBX_GHST:fresh' ;;
        *) cmd="printf 'MBX_GHST:stale\\n'" ;;
    esac
    printf 'MBX2\t%s\tRESULT\t%s\t1\t%s\n' "$id" "$gen" "$cmd"
}

while IFS= read -r line; do
    IFS=$'\t' read -r -a fields <<<"$line"
    magic=${fields[0]-}
    id=${fields[1]-}
    kind=${fields[2]-}
    case $magic-$kind in
        MBX1-PING)
            printf 'MBX1\t%s\tPONG\n' "$id"
            ;;
        MBX1-PROMPT)
            printf 'MBX1\t%s\tPROMPT\t> \n' "$id"
            ;;
        MBX2-PING)
            printf 'MBX2\t%s\tPONG\n' "$id"
            ;;
        MBX2-RECORD|MBX2-CANCEL)
            printf 'MBX2\t%s\tACK\n' "$id"
            ;;
        MBX2-QUERY)
            gen=${fields[3]-}
            text=${fields[5]-}
            if [[ -z $held_id ]]; then
                held_id=$id
                held_gen=$gen
                held_text=$text
                continue
            fi
            result_for "$held_id" "$held_gen" "$held_text"
            held_id=
            held_gen=
            held_text=
            result_for "$id" "$gen" "$text"
            ;;
        *)
            printf 'MBX2\t%s\tERROR\tinvalid\n' "${id:-0}"
            ;;
    esac
done
