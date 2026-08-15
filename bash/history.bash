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

_mbx_history_prompt() {
    local entry number
    local history_number=

    [[ ${_MBX_HISTORY_ENABLED:-0} == 1 ]] || return 0
    [[ ${_MBX_ENGINE_READY:-0} == 1 ]] || return 0

    # `history 1` prints the newest admitted entry as `  N  text` (right-aligned
    # number, two-space separator). Bash stores the folded single-line form,
    # which is exactly the admission authority; a missing entry means Bash did
    # not admit one (history off, HISTCONTROL/HISTIGNORE drop) and there is
    # nothing to record.
    entry=$(history 1 2>/dev/null) || entry=
    entry=${entry##*  }
    [[ -n $entry ]] || return 0
    [[ -n ${HISTCMD:-} ]] && history_number=$HISTCMD
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

    [[ -n $completed_at ]] || return 1
    _mbx_history_iso_utc "${completed_at%%.*}" || return 1
    completed_at=$REPLY

    _mbx_deadline_after "${MBX_HISTORY_TIMEOUT:-0.10}" || return 1
    deadline=$REPLY
    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID

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
