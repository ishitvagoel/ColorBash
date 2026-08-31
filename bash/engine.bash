# shellcheck shell=bash
# Native renderer transports and coprocess lifecycle. Successful prompt adapters
# return their rendered value through REPLY and never mutate PS1.

_mbx_clock_now_us() {
    local timestamp=${EPOCHREALTIME:-}
    local seconds fraction

    [[ $timestamp =~ ^[0-9]+\.[0-9]+$ ]] || return 1
    seconds=${timestamp%%.*}
    fraction=${timestamp#*.}
    fraction=${fraction:0:6}
    while ((${#fraction} < 6)); do
        fraction+=0
    done
    REPLY=$((10#$seconds * 1000000 + 10#$fraction))
}

_mbx_timeout_to_us() {
    local timeout=${1-}
    local whole fraction

    if [[ $timeout =~ ^([0-9]+)(\.([0-9]+))?$ ]]; then
        whole=${BASH_REMATCH[1]}
        fraction=${BASH_REMATCH[3]:-}
    elif [[ $timeout =~ ^\.([0-9]+)$ ]]; then
        whole=0
        fraction=${BASH_REMATCH[1]}
    else
        return 1
    fi
    fraction=${fraction:0:6}
    while ((${#fraction} < 6)); do
        fraction+=0
    done
    REPLY=$((10#$whole * 1000000 + 10#$fraction))
}

_mbx_deadline_after() {
    local timeout_us now_us

    _mbx_timeout_to_us "$1" || return 1
    timeout_us=$REPLY
    _mbx_clock_now_us || return 1
    now_us=$REPLY
    REPLY=$((now_us + timeout_us))
}

_mbx_render_deadline_start() {
    _mbx_deadline_after "${MBX_RENDER_TIMEOUT:-0.10}" || _mbx_deadline_after 0.10
}

_mbx_deadline_remaining() {
    local deadline=${1-}
    local now_us remaining_us

    [[ $deadline =~ ^[0-9]+$ ]] || return 1
    _mbx_clock_now_us || return 1
    now_us=$REPLY
    ((now_us < deadline)) || return 1
    remaining_us=$((deadline - now_us))
    printf -v REPLY '%d.%06d' "$((remaining_us / 1000000))" "$((remaining_us % 1000000))"
}

_mbx_deadline_slice() {
    local deadline=$1
    local maximum_us=$2
    local remaining

    _mbx_deadline_remaining "$deadline" || return 1
    remaining=$REPLY
    _mbx_timeout_to_us "$remaining" || return 1
    if ((REPLY > maximum_us)); then
        printf -v REPLY '0.%06d' "$maximum_us"
    else
        REPLY=$remaining
    fi
}

_mbx_exchange_deadline() {
    local render_deadline=${1-}
    local ipc_deadline

    _mbx_deadline_after "${MBX_IPC_TIMEOUT:-0.10}" || _mbx_deadline_after 0.10 || return 1
    ipc_deadline=$REPLY
    if [[ $render_deadline =~ ^[0-9]+$ ]] && ((render_deadline < ipc_deadline)); then
        REPLY=$render_deadline
    else
        REPLY=$ipc_deadline
    fi
}

_mbx_defer_child() {
    local pid=${1-}
    [[ $pid =~ ^[0-9]+$ ]] || return 0
    _MBX_DEFERRED_CHILD_PIDS="${_MBX_DEFERRED_CHILD_PIDS:+$_MBX_DEFERRED_CHILD_PIDS }$pid"
}

_mbx_reap_children() {
    local pid
    local remaining=

    for pid in ${_MBX_DEFERRED_CHILD_PIDS:-}; do
        if kill -0 "$pid" 2>/dev/null; then
            remaining="${remaining:+$remaining }$pid"
        else
            wait "$pid" 2>/dev/null || true
        fi
    done
    _MBX_DEFERRED_CHILD_PIDS=$remaining
}

_mbx_terminate_child() {
    local pid=${1-}

    [[ $pid =~ ^[0-9]+$ ]] || return 0
    if kill -0 "$pid" 2>/dev/null; then
        kill -TERM "$pid" 2>/dev/null || true
        if kill -0 "$pid" 2>/dev/null; then
            kill -KILL "$pid" 2>/dev/null || true
        fi
    fi
    # Never put an unbounded wait on the prompt path. Bash reaps background
    # children asynchronously; a later prompt collects the retained status once
    # kill -0 confirms that waiting cannot block.
    _mbx_defer_child "$pid"
}

_mbx_wait_child_until() {
    local pid=$1
    local deadline=$2
    local status=0

    while kill -0 "$pid" 2>/dev/null; do
        _mbx_deadline_remaining "$deadline" >/dev/null || return 1
    done
    wait "$pid" 2>/dev/null || status=$?
    REPLY=$status
}

# Wait for a child until the deadline. On timeout, terminate it and return 1
# so callers cannot treat a killed helper's leftover payload as success
# (M-067). On success, REPLY is the child's exit status.
_mbx_wait_or_kill_child() {
    local pid=$1
    local deadline=$2

    if _mbx_wait_child_until "$pid" "$deadline"; then
        return 0
    fi
    _mbx_terminate_child "$pid"
    return 1
}

# Shared bind -x job-control isolation (M-049). Features that spawn from a
# keystroke callback must suspend monitor/notify and restore them on every
# return path. One pair of flags is enough: these widgets do not nest.
_MBX_JOBS_SAVED_MONITOR=0
_MBX_JOBS_SAVED_NOTIFY=0

_mbx_jobs_suspend() {
    _MBX_JOBS_SAVED_MONITOR=0
    _MBX_JOBS_SAVED_NOTIFY=0
    [[ $- == *m* ]] && _MBX_JOBS_SAVED_MONITOR=1
    [[ $- == *b* ]] && _MBX_JOBS_SAVED_NOTIFY=1
    set +m
    set +b
}

_mbx_jobs_restore() {
    ((${_MBX_JOBS_SAVED_NOTIFY:-0} == 1)) && set -b
    ((${_MBX_JOBS_SAVED_MONITOR:-0} == 1)) && set -m
    _MBX_JOBS_SAVED_NOTIFY=0
    _MBX_JOBS_SAVED_MONITOR=0
}

# Below-prompt paint (M-065 / ADR 0015). IND reserves rows so a later DECSC
# cannot be invalidated by the draw's own scroll. Callers write to /dev/tty.
_mbx_tty_columns() {
    local cols=${COLUMNS:-}
    [[ $cols =~ ^[0-9]+$ ]] && ((cols > 1)) || cols=80
    REPLY=$((cols - 1))
}

_mbx_tty_reserve_rows() {
    local count=${1:-0}
    local index pad=

    ((count > 0)) || return 0
    for ((index = 0; index < count; index++)); do
        pad+=$'\eD'
    done
    printf '%s\e[%dA' "$pad" "$count" >/dev/tty 2>/dev/null || true
}

_mbx_tty_save_cursor() {
    printf '\e7' >/dev/tty 2>/dev/null || true
}

_mbx_tty_restore_cursor() {
    printf '\e8' >/dev/tty 2>/dev/null || true
}

_mbx_tty_erase_below() {
    printf '\e[J' >/dev/tty 2>/dev/null || true
}

# Visible-width clamp for a tty row. CSI SGR runs and SOH/STX markers do not
# consume columns. Non-ASCII scalars count as two columns (conservative).
# Indexing is LC_ALL=C bytes: `${#var}` / `${var:i:1}` walk Unicode scalars
# only in a UTF-8 locale, and the Bash-matrix CI containers are C/POSIX.
_mbx_tty_clamp_row() {
    local text=$1
    local max=${2:-79}
    local out= width=0 index=0 ch code w seq len remaining
    local LC_ALL=C

    ((max > 0)) || {
        REPLY=
        return 0
    }
    while ((index < ${#text})); do
        ch=${text:index:1}
        printf -v code '%d' "'$ch"
        if ((code < 0)); then
            code=$((code + 256))
        fi
        if ((code == 1 || code == 2)); then
            index=$((index + 1))
            continue
        fi
        if ((code == 27)) && [[ ${text:index+1:1} == '[' ]]; then
            out+=$'\e['
            index=$((index + 2))
            while ((index < ${#text})); do
                ch=${text:index:1}
                out+=$ch
                index=$((index + 1))
                [[ $ch == m ]] && break
            done
            continue
        fi
        if ((code < 128)); then
            w=1
            seq=$ch
            len=1
        else
            w=2
            if ((code >= 240)); then
                len=4
            elif ((code >= 224)); then
                len=3
            elif ((code >= 192)); then
                len=2
            else
                len=1
            fi
            remaining=$((${#text} - index))
            if ((len > remaining)); then
                len=$remaining
            fi
            seq=${text:index:len}
        fi
        if ((width + w > max)); then
            break
        fi
        out+=$seq
        width=$((width + w))
        index=$((index + len))
    done
    REPLY=$out
}


# Reads one bounded, already-LF-terminated line (an optional CR is stripped)
# from a coprocess or process-substitution fd, tolerant of a read that hits
# its deadline with a partial line already in `REPLY` (status 1). Used by any
# caller that must skip a stale reply and keep reading later frames from the
# same fd (ADR 0011 stale-generation skip): ghost QUERY/RESULT and highlight
# HIGHLIGHT/STYLED both need this, so it lives here rather than inside either
# feature module.
_mbx_engine_read_line() {
    local fd=$1
    local deadline=$2
    local timeout status=0
    local LC_ALL=C

    _mbx_deadline_remaining "$deadline" || return 1
    timeout=$REPLY
    REPLY=
    IFS= read -r -t "$timeout" -n 65536 -u "$fd" REPLY || status=$?
    case $status in
        0 | 1)
            [[ -n $REPLY || $status == 0 ]] || return 1
            if [[ $REPLY == *$'\r' ]]; then
                REPLY=${REPLY%$'\r'}
            fi
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

_mbx_read_bounded_response() {
    local fd=$1
    local deadline=$2
    local response= chunk= timeout before_lf
    local read_status=0
    local length chunk_length requested
    local LC_ALL=C

    # Empty -d makes NUL an observable delimiter. Without it, Bash silently
    # discards NUL bytes and a raw-wire acquisition bound cannot be enforced.
    _mbx_deadline_remaining "$deadline" || return 1
    timeout=$REPLY
    IFS= read -r -d '' -t "$timeout" -n 1 response <&"$fd" || read_status=$?
    case $read_status in
        0)
            ((${#response} == 1)) || return 1
            ;;
        1)
            ((${#response} == 0)) || return 1
            ;;
        *) return 1 ;;
    esac

    while :; do
        length=${#response}
        if [[ $response == *$'\n'* ]]; then
            # A frame terminator must be the final and only LF acquired for this
            # response. Normalize its optional CR before applying the payload
            # limit.
            [[ $response == *$'\n' ]] || return 1
            before_lf=${response%$'\n'}
            [[ $before_lf != *$'\n'* ]] || return 1
            if [[ $before_lf == *$'\r' ]]; then
                before_lf=${before_lf%$'\r'}
            fi
            ((${#before_lf} <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || return 1
            REPLY=$before_lf
            return 0
        fi

        if ((read_status == 1)); then
            # EOF is a valid terminator, but a raw trailing CR is payload and is
            # left for the protocol decoder to reject.
            ((length <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || return 1
            REPLY=$response
            return 0
        fi

        ((length < _MBX_PROTOCOL_MAX_MESSAGE_BYTES + 2)) || return 1
        if ((length == _MBX_PROTOCOL_MAX_MESSAGE_BYTES + 1)) && \
            [[ $response != *$'\r' ]]; then
            return 1
        fi

        requested=$((_MBX_PROTOCOL_MAX_MESSAGE_BYTES + 2 - length))
        # `read` itself cannot be interrupted while it is copying immediately
        # available bytes. Keep each builtin call small enough that deadline
        # checks remain effective even for a peer continuously filling the pipe.
        ((requested <= 4096)) || requested=4096
        _mbx_deadline_slice "$deadline" 500 || return 1
        timeout=$REPLY
        chunk=
        read_status=0
        IFS= read -r -d '' -t "$timeout" -n "$requested" chunk <&"$fd" || \
            read_status=$?
        chunk_length=${#chunk}
        # With an empty delimiter, status 0 before the requested count means a
        # NUL was consumed. It is forbidden even though Bash cannot store it.
        if ((read_status == 0 && chunk_length < requested)); then
            return 1
        fi
        case $read_status in
            0|1) ;;
            *)
                ((read_status > 128)) || return 1
                ;;
        esac
        response+=$chunk
    done
}

_mbx_engine_stop() {
    local child_pid=${_MBX_ENGINE_CHILD_PID:-}

    if [[ -n ${_MBX_ENGINE_IN_FD:-} ]]; then
        exec {_MBX_ENGINE_IN_FD}>&-
        unset _MBX_ENGINE_IN_FD
    fi
    if [[ -n ${_MBX_ENGINE_OUT_FD:-} ]]; then
        exec {_MBX_ENGINE_OUT_FD}<&-
        unset _MBX_ENGINE_OUT_FD
    fi
    unset _MBX_ENGINE_CHILD_PID
    _MBX_ENGINE_READY=0
    _mbx_terminate_child "$child_pid"
    _mbx_reap_children
}

_mbx_engine_write() {
    (($# == 1 || $# == 2)) || return 2

    local request=$1
    local render_deadline=${2-}
    local deadline writer_pid writer_status

    [[ -n ${_MBX_ENGINE_CHILD_PID:-} && \
        -n ${_MBX_ENGINE_IN_FD:-} && \
        -n ${_MBX_ENGINE_OUT_FD:-} ]] || return 1
    kill -0 "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || return 1
    _mbx_protocol_validate_line "$request" || return 1
    _mbx_exchange_deadline "$render_deadline" || return 1
    deadline=$REPLY
    _mbx_deadline_remaining "$deadline" >/dev/null || return 1

    # A background builtin write keeps a peer that stops reading from consuming
    # the complete render deadline. Its PID is always bounded and collected.
    # `set +m` alone does not stop Bash announcing "[N] PID" for this job when
    # the caller runs from a bind -x keystroke callback (confirmed: it does
    # not happen from PROMPT_COMMAND, but does from self-insert) — the
    # announcement goes to the shell's own stderr, not the backgrounded
    # command's, so only wrapping the whole group's stderr suppresses it
    # (M-063).
    { (
        trap '' PIPE
        printf '%s\n' "$request" >&"$_MBX_ENGINE_IN_FD" 2>/dev/null
    ) & } 2>/dev/null
    writer_pid=$!
    if ! _mbx_wait_child_until "$writer_pid" "$deadline"; then
        _mbx_terminate_child "$writer_pid"
        return 1
    fi
    writer_status=$REPLY
    ((writer_status == 0)) || return 1
}

_mbx_engine_read() {
    (($# == 0 || $# == 1)) || return 2

    local render_deadline=${1-}
    local deadline

    [[ -n ${_MBX_ENGINE_CHILD_PID:-} && \
        -n ${_MBX_ENGINE_OUT_FD:-} ]] || return 1
    kill -0 "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || return 1
    _mbx_exchange_deadline "$render_deadline" || return 1
    deadline=$REPLY
    _mbx_read_bounded_response "$_MBX_ENGINE_OUT_FD" "$deadline"
}

_mbx_engine_exchange() {
    (($# == 1 || $# == 2)) || return 2

    local request=$1
    local render_deadline=${2-}
    local response= deadline writer_pid writer_status
    local read_ok=0

    [[ -n ${_MBX_ENGINE_CHILD_PID:-} && \
        -n ${_MBX_ENGINE_IN_FD:-} && \
        -n ${_MBX_ENGINE_OUT_FD:-} ]] || return 1
    kill -0 "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || return 1
    _mbx_protocol_validate_line "$request" || return 1
    _mbx_exchange_deadline "$render_deadline" || return 1
    deadline=$REPLY
    _mbx_deadline_remaining "$deadline" >/dev/null || return 1

    # A background builtin write keeps a peer that stops reading from consuming
    # the complete render deadline. Its PID is always bounded and collected.
    # `set +m` alone does not stop Bash announcing "[N] PID" for this job when
    # the caller runs from a bind -x keystroke callback (confirmed: it does
    # not happen from PROMPT_COMMAND, but does from self-insert) — the
    # announcement goes to the shell's own stderr, not the backgrounded
    # command's, so only wrapping the whole group's stderr suppresses it
    # (M-063).
    { (
        trap '' PIPE
        printf '%s\n' "$request" >&"$_MBX_ENGINE_IN_FD" 2>/dev/null
    ) & } 2>/dev/null
    writer_pid=$!

    if _mbx_read_bounded_response "$_MBX_ENGINE_OUT_FD" "$deadline"; then
        response=$REPLY
        read_ok=1
    fi

    if ! _mbx_wait_child_until "$writer_pid" "$deadline"; then
        _mbx_terminate_child "$writer_pid"
        return 1
    fi
    writer_status=$REPLY
    ((read_ok == 1 && writer_status == 0)) || return 1
    REPLY=$response
}

_mbx_engine_ping() {
    local request_id response deadline

    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID
    _mbx_exchange_deadline || return 1
    deadline=$REPLY
    _mbx_protocol_encode_ping "$request_id"
    _mbx_engine_exchange "$REPLY" "$deadline" || return 1
    response=$REPLY
    _mbx_protocol_decode_pong \
        "$request_id" "$response" _mbx_deadline_remaining "$deadline"
}

_mbx_engine_start() {
    local original_out original_in
    local _mbx_saved_monitor=0 _mbx_saved_notify=0

    # Starting is idempotent from the caller's perspective and cannot orphan a
    # previously owned coprocess.
    _mbx_engine_stop
    _MBX_REQUEST_ID=0
    _mbx_coprocess_requested || return 1

    # Interactive Bash defaults to monitor mode. A long-lived coproc listed as
    # job #1 shares the shell process group, so Ctrl+C at the prompt SIGINTs
    # the helper and the `[1]+ Interrupt` line can steal the next command's
    # first byte (M-051). Suspend monitor/notify only around spawn, ignore
    # INT/QUIT/TSTP across exec (POSIX SIG_IGN is inherited), then restore
    # the caller's flags. Do not leave monitor off for the session.
    [[ $- == *m* ]] && _mbx_saved_monitor=1
    [[ $- == *b* ]] && _mbx_saved_notify=1
    set +m
    set +b
    coproc _MBX_ENGINE_COPROC {
        trap '' INT QUIT TSTP
        exec "$MBX_BIN" serve --stdio 2>/dev/null
    }
    _MBX_ENGINE_CHILD_PID=${_MBX_ENGINE_COPROC_PID-}
    ((_mbx_saved_notify == 1)) && set -b
    ((_mbx_saved_monitor == 1)) && set -m
    if [[ -n ${_MBX_ENGINE_CHILD_PID:-} ]]; then
        builtin disown "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || true
    fi

    if [[ -z ${_MBX_ENGINE_CHILD_PID:-} || \
        -z ${_MBX_ENGINE_COPROC[0]:-} || \
        -z ${_MBX_ENGINE_COPROC[1]:-} ]]; then
        _mbx_engine_stop
        return 1
    fi

    exec {_MBX_ENGINE_OUT_FD}<&"${_MBX_ENGINE_COPROC[0]}"
    exec {_MBX_ENGINE_IN_FD}>&"${_MBX_ENGINE_COPROC[1]}"
    original_out=${_MBX_ENGINE_COPROC[0]}
    original_in=${_MBX_ENGINE_COPROC[1]}
    exec {original_out}<&-
    exec {original_in}>&-
    unset _MBX_ENGINE_COPROC _MBX_ENGINE_COPROC_PID

    if _mbx_engine_ping; then
        _MBX_ENGINE_READY=1
        return 0
    fi
    _mbx_engine_stop
    return 1
}

_mbx_prompt_from_coprocess() {
    (($# == 4)) || return 2

    local status=$1
    local duration_ms=$2
    local cwd=$3
    local flags=$4
    local request_id response deadline

    [[ ${_MBX_ENGINE_READY:-0} == 1 ]] || return 1
    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID
    _mbx_exchange_deadline "${_MBX_RENDER_DEADLINE_US:-}" || return 1
    deadline=$REPLY
    _mbx_protocol_encode_prompt \
        "$request_id" "$cwd" "$status" "$duration_ms" "$flags" \
        _mbx_deadline_remaining "$deadline" || return 1
    _mbx_engine_exchange "$REPLY" "$deadline" || return 1
    response=$REPLY
    _mbx_protocol_decode_prompt \
        "$request_id" "$response" _mbx_deadline_remaining "$deadline"
}

_mbx_prompt_per_call() {
    (($# == 4)) || return 2

    local status=$1
    local duration_ms=$2
    local cwd=$3
    local flags=$4
    local deadline=${_MBX_RENDER_DEADLINE_US:-}
    local output= output_fd child_pid child_status
    local read_ok=0
    local -a args=(prompt --cwd "$cwd" --status "$status" --flags "$flags")

    if [[ $duration_ms != - ]]; then
        args+=(--duration-ms "$duration_ms")
    fi
    if [[ ! $deadline =~ ^[0-9]+$ ]]; then
        _mbx_render_deadline_start || return 1
        deadline=$REPLY
    fi
    _mbx_deadline_remaining "$deadline" >/dev/null || return 1

    exec {output_fd}< <(exec "$MBX_BIN" "${args[@]}" 2>/dev/null)
    child_pid=$!
    if _mbx_read_bounded_response "$output_fd" "$deadline"; then
        output=$REPLY
        read_ok=1
    fi
    exec {output_fd}<&-

    if ! _mbx_wait_child_until "$child_pid" "$deadline"; then
        _mbx_terminate_child "$child_pid"
        return 1
    fi
    child_status=$REPLY
    ((read_ok == 1 && child_status == 0)) || return 1
    REPLY=$output
}
