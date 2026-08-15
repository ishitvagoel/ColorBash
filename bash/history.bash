# shellcheck shell=bash
# Opt-in history recorder. Runs at the prompt boundary, reads the last
# Bash-admitted history entry (folded form) with `history 1`, and enqueues it
# to the helper over MBX2 with a bounded deadline. Records are enhancement
# data only: any failure, full queue, or expired deadline drops the record and
# increments a command-text-free counter; the prompt and shell are never
# blocked.

_mbx_history_init() {
    _MBX_HISTORY_ENABLED=0
    [[ ${MBX_HISTORY:-0} == 1 ]] || return 0
    [[ ${MBX_DISABLE_RENDERER:-0} != 1 ]] || return 0
    [[ -x ${MBX_BIN:-} ]] || return 0
    [[ ${_MBX_ENGINE_READY:-0} == 1 ]] || return 0
    _MBX_HISTORY_SESSION_ID=$(_mbx_history_session_id)
    _MBX_HISTORY_SEQUENCE=0
    _MBX_HISTORY_LAST_ENTRY=
    _MBX_HISTORY_LAST_NUMBER=
    _MBX_HISTORY_DROPPED=0
    _MBX_HISTORY_ENABLED=1
}

_mbx_history_session_id() {
    local now=${EPOCHREALTIME:-}
    [[ -n $now ]] || now=0
    local seconds=${now%%.*}
    local fraction=${now#*.}000000
    fraction=${fraction:0:6}
    printf '%s%06d-%d-%d-%d\n' \
        "$seconds" "$fraction" "$BASHPID" "$RANDOM" "$RANDOM"
}

_mbx_history_parse_latest() {
    local line=
    line=$(history 1 2>/dev/null) || line=
    if [[ $line =~ ^[[:space:]]*([0-9]+)[[:space:]][[:space:]](.*)$ ]]; then
        _MBX_HISTORY_LATEST_NUMBER=${BASH_REMATCH[1]}
        REPLY=${BASH_REMATCH[2]}
        return 0
    fi
    return 1
}

_mbx_history_prompt() {
    local entry history_number=

    [[ ${_MBX_HISTORY_ENABLED:-0} == 1 ]] || return 0
    [[ ${_MBX_ENGINE_READY:-0} == 1 ]] || return 0
    # The first prompt is not a command-completion boundary; HISTFILE may already
    # contain prior sessions. Snapshot the drop key only, then record after
    # commands complete.
    if [[ ${_MBX_HISTORY_SAW_PROMPT:-0} != 1 ]]; then
        _MBX_HISTORY_SAW_PROMPT=1
        if _mbx_history_parse_latest; then
            _MBX_HISTORY_LAST_ENTRY=$REPLY
            _MBX_HISTORY_LAST_NUMBER=$_MBX_HISTORY_LATEST_NUMBER
        fi
        return 0
    fi

    # `history 1` prints `  N  text` (right-aligned list number, exactly two
    # spaces, then the folded command). The list number is the drop key: HISTCMD
    # is not a stable identifier and may be unset or still advance while history
    # is off. Greedy `${var##*  }` would also eat a user-typed leading space.
    _mbx_history_parse_latest || return 0
    entry=$REPLY
    history_number=$_MBX_HISTORY_LATEST_NUMBER
    [[ -n $entry ]] || return 0
    if [[ -n $_MBX_HISTORY_LAST_ENTRY && \
        "$entry" == "$_MBX_HISTORY_LAST_ENTRY" && \
        "$history_number" == "$_MBX_HISTORY_LAST_NUMBER" ]]; then
        return 0
    fi

    if _mbx_history_excluded "$entry"; then
        _MBX_HISTORY_LAST_ENTRY=$entry
        _MBX_HISTORY_LAST_NUMBER=$history_number
        return 0
    fi

    _MBX_HISTORY_SEQUENCE=$((_MBX_HISTORY_SEQUENCE + 1))
    if _mbx_history_record \
        "$entry" "$history_number" "${_MBX_LAST_STATUS:-0}" "${_MBX_LAST_DURATION_MS:--}"; then
        _MBX_HISTORY_LAST_ENTRY=$entry
        _MBX_HISTORY_LAST_NUMBER=$history_number
    else
        _MBX_HISTORY_DROPPED=$((_MBX_HISTORY_DROPPED + 1))
    fi
}

_mbx_history_excluded() {
    local entry=$1
    local -a patterns=()
    local pattern

    [[ -n ${MBX_HISTORY_EXCLUDE:-} ]] || return 1
    IFS=: read -ra patterns <<<"${MBX_HISTORY_EXCLUDE}"
    for pattern in "${patterns[@]}"; do
        [[ -n $pattern ]] || continue
        case $entry in
            $pattern) return 0 ;;
        esac
    done
    return 1
}

_mbx_history_ack_bench_enabled() {
    [[ ${MBX_HISTORY:-0} == 1 && ${MBX_HISTORY_ACK_BENCH:-0} == 1 ]]
}

_mbx_history_ack_sample_path() {
    local data_home=${XDG_DATA_HOME:-}
    if [[ -z $data_home ]]; then
        data_home=${HOME:-}/.local/share
    fi
    REPLY=$data_home/mbx/history-ack-samples
}

_mbx_history_ack_sample() {
    local elapsed_us=$1
    local path dir

    _mbx_history_ack_bench_enabled || return 0
    _mbx_history_ack_sample_path
    path=$REPLY
    dir=${path%/*}
    [[ -d $dir ]] || mkdir -p "$dir" 2>/dev/null || return 0
    if [[ ! -f $path ]]; then
        # umask in a subshell so the interactive shell's umask is unchanged.
        ( umask 077; : >"$path" ) 2>/dev/null || return 0
    fi
    printf '%u\n' "$elapsed_us" >>"$path" 2>/dev/null || true
}

_mbx_history_record() {
    local entry=$1
    local history_number=$2
    local status=$3
    local duration=$4
    local request_id= deadline= response=
    local completed_at=${EPOCHREALTIME:-}
    local cwd=${PWD:-}
    local host=${HOSTNAME:-}
    local user=${USER:-}
    local read_ok=0
    local ack_started_us= ack_elapsed_us=

    [[ -n $completed_at ]] || return 1
    _mbx_history_iso_utc "${completed_at%%.*}" || return 1
    completed_at=$REPLY

    _mbx_deadline_after "${MBX_HISTORY_TIMEOUT:-0.10}" || return 1
    deadline=$REPLY
    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID

    if _mbx_history_ack_bench_enabled && _mbx_now_us; then
        ack_started_us=$REPLY
    fi

    _mbx_protocol_encode_history_record \
        "$request_id" "$_MBX_HISTORY_SESSION_ID" "$_MBX_HISTORY_SEQUENCE" \
        "$history_number" "$entry" "$cwd" "$completed_at" \
        "$status" "$duration" "$host" "$user" || return 1

    if _mbx_engine_exchange "$REPLY" "$deadline"; then
        response=$REPLY
        _mbx_protocol_decode_history_ack "$request_id" "$response" && read_ok=1
    fi
    if ((read_ok != 1)); then
        # A timed-out or malformed history exchange can leave the helper's
        # response unread in the coprocess pipe. Stop the engine so the prompt
        # path degrades cleanly instead of consuming a stale frame.
        _mbx_engine_stop
        return 1
    fi
    if [[ -n $ack_started_us ]] && _mbx_now_us; then
        ack_elapsed_us=$((REPLY - ack_started_us))
        if ((ack_elapsed_us >= 0)); then
            _mbx_history_ack_sample "$ack_elapsed_us"
        fi
    fi
    return 0
}

_mbx_history_iso_utc() {
    # Pure-builtin epoch-to-ISO conversion (Howard Hinnant civil algorithm),
    # keeping the prompt hot path free of external processes.
    local seconds=$1
    local days hours minutes secs
    local z era doe yoe year doy mp
    local day month

    ((seconds >= 0)) || return 1
    days=$((seconds / 86400))
    hours=$(((seconds % 86400) / 3600))
    minutes=$((((seconds % 86400) % 3600) / 60))
    secs=$((seconds % 60))

    z=$((days + 719468))
    if ((z >= 0)); then
        era=$((z / 146097))
    else
        era=$(((z - 146096) / 146097))
    fi
    doe=$((z - era * 146097))
    yoe=$(((doe - doe / 1460 + doe / 36524 - doe / 146096) / 365))
    year=$((yoe + era * 400))
    doy=$((doe - (365 * yoe + yoe / 4 - yoe / 100)))
    mp=$(((5 * doy + 2) / 153))
    day=$((doy - (153 * mp + 2) / 5 + 1))
    month=$((mp + (mp < 10 ? 3 : -9)))
    ((month <= 2)) && year=$((year + 1))
    printf -v REPLY '%04d-%02d-%02dT%02d:%02d:%02dZ' \
        "$year" "$month" "$day" "$hours" "$minutes" "$secs"
}

_mbx_history_install_hooks() {
    _mbx_history_init
}
