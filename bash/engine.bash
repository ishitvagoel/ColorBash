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
    (
        trap '' PIPE
        printf '%s\n' "$request" >&"$_MBX_ENGINE_IN_FD" 2>/dev/null
    ) &
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

    # Starting is idempotent from the caller's perspective and cannot orphan a
    # previously owned coprocess.
    _mbx_engine_stop
    _MBX_REQUEST_ID=0
    _mbx_coprocess_requested || return 1

    coproc _MBX_ENGINE_COPROC { exec "$MBX_BIN" serve --stdio 2>/dev/null; }
    _MBX_ENGINE_CHILD_PID=$_MBX_ENGINE_COPROC_PID
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
