#!/usr/bin/env bash
set -euo pipefail

case ${1:-} in
    serve)
        IFS= read -r request
        IFS=$'\t' read -r magic request_id kind extra <<<"$request"
        [[ $magic == MBX1 && $kind == PING && -z $extra ]]
        printf 'MBX1\t%s\tPONG\n' "$request_id"
        IFS= read -r request
        if [[ -n ${MBX_STALL_SERVE_PROMPT_MARKER:-} ]]; then
            : >"$MBX_STALL_SERVE_PROMPT_MARKER"
        fi
        if [[ ${MBX_STALL_RESPONSE_MODE:-} == percent-heavy ]]; then
            IFS=$'\t' read -r magic request_id kind _ <<<"$request"
            [[ $magic == MBX1 && $kind == PROMPT ]]
            printf 'MBX1\t%s\tPROMPT\t' "$request_id"
            printf '%%41%.0s' {1..21800}
            printf '\n'
            if [[ -n ${MBX_STALL_RESPONSE_MARKER:-} ]]; then
                : >"$MBX_STALL_RESPONSE_MARKER"
            fi
        fi
        exec sleep 60
        ;;
    prompt)
        if [[ -n ${MBX_STALL_PROMPT_MARKER:-} ]]; then
            : >"$MBX_STALL_PROMPT_MARKER"
        fi
        exec sleep 60
        ;;
    *)
        exit 2
        ;;
esac
