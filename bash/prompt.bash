# shellcheck shell=bash
# Prompt transport and rendering selection. This file intentionally uses only
# Bash builtins on the coprocess hot path.

_mbx_escape_field() {
    local value=${1-}
    value=${value//%/%25}
    value=${value//$'\t'/%09}
    value=${value//$'\n'/%0A}
    value=${value//$'\r'/%0D}
    REPLY=$value
}

_mbx_unescape_field() {
    local value=${1-}
    value=${value//%09/$'\t'}
    value=${value//%0A/$'\n'}
    value=${value//%0D/$'\r'}
    value=${value//%25/%}
    REPLY=$value
}

_mbx_prompt_flags() {
    local flags=0
    if [[ ! -t 1 || ${TERM:-dumb} == dumb || -n ${NO_COLOR+x} || ${MBX_COLOR:-auto} == never ]]; then
        ((flags |= 1))
    fi
    case ${MBX_ICONS:-auto} in
        never|ascii) ((flags |= 2)) ;;
        nerd) ((flags |= 4)) ;;
    esac
    if [[ -n ${SSH_CONNECTION:-} || -n ${SSH_TTY:-} ]]; then
        ((flags |= 8))
    fi
    if [[ ${MBX_PRODUCTION_CONTEXT:-0} == 1 ]]; then
        ((flags |= 16))
    fi
    if [[ ${MBX_DISABLE_GIT:-0} == 1 ]]; then
        ((flags |= 32))
    fi
    REPLY=$flags
}

_mbx_engine_stop() {
    if [[ -n ${_MBX_ENGINE_IN_FD:-} ]]; then
        exec {_MBX_ENGINE_IN_FD}>&-
        unset _MBX_ENGINE_IN_FD
    fi
    if [[ -n ${_MBX_ENGINE_OUT_FD:-} ]]; then
        exec {_MBX_ENGINE_OUT_FD}<&-
        unset _MBX_ENGINE_OUT_FD
    fi
    if [[ -n ${_MBX_ENGINE_CHILD_PID:-} ]]; then
        if kill -0 "$_MBX_ENGINE_CHILD_PID" 2>/dev/null; then
            kill "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || true
        fi
        wait "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || true
        unset _MBX_ENGINE_CHILD_PID
    fi
    _MBX_ENGINE_READY=0
}

_mbx_engine_ping() {
    local id response_magic response_id response_kind extra
    [[ -n ${_MBX_ENGINE_CHILD_PID:-} ]] && kill -0 "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || return 1
    ((_MBX_REQUEST_ID += 1))
    id=$_MBX_REQUEST_ID
    (trap '' PIPE; printf 'MBX1\t%s\tPING\n' "$id" >&"$_MBX_ENGINE_IN_FD") || return 1
    IFS=$'\t' read -r -t "${MBX_IPC_TIMEOUT:-0.10}" \
        response_magic response_id response_kind extra <&"$_MBX_ENGINE_OUT_FD" || return 1
    [[ $response_magic == MBX1 && $response_id == "$id" && $response_kind == PONG && -z $extra ]]
}

_mbx_engine_start() {
    _MBX_ENGINE_READY=0
    _MBX_REQUEST_ID=0
    [[ ${MBX_DISABLE_RENDERER:-0} != 1 ]] || return 1
    [[ ${MBX_IPC_MODE:-auto} != off ]] || return 1
    [[ -x ${MBX_BIN:-} ]] || return 1

    case ${MBX_IPC_MODE:-auto} in
        auto|coprocess)
            coproc _MBX_ENGINE_COPROC { exec "$MBX_BIN" serve --stdio 2>/dev/null; }
            _MBX_ENGINE_CHILD_PID=$_MBX_ENGINE_COPROC_PID
            exec {_MBX_ENGINE_OUT_FD}<&"${_MBX_ENGINE_COPROC[0]}"
            exec {_MBX_ENGINE_IN_FD}>&"${_MBX_ENGINE_COPROC[1]}"
            local original_out=${_MBX_ENGINE_COPROC[0]}
            local original_in=${_MBX_ENGINE_COPROC[1]}
            exec {original_out}<&-
            exec {original_in}>&-
            unset _MBX_ENGINE_COPROC _MBX_ENGINE_COPROC_PID
            if _mbx_engine_ping; then
                _MBX_ENGINE_READY=1
                return 0
            fi
            _mbx_engine_stop
            [[ ${MBX_IPC_MODE:-auto} == auto ]] || return 1
            ;;
        per-call) return 0 ;;
        *) return 1 ;;
    esac
}

_mbx_prompt_from_coprocess() {
    local status=$1
    local duration_ms=$2
    local id flags escaped_cwd response_magic response_id response_kind payload extra
    [[ -n ${_MBX_ENGINE_CHILD_PID:-} ]] && kill -0 "$_MBX_ENGINE_CHILD_PID" 2>/dev/null || return 1
    _mbx_prompt_flags
    flags=$REPLY
    _mbx_escape_field "$PWD"
    escaped_cwd=$REPLY
    ((_MBX_REQUEST_ID += 1))
    id=$_MBX_REQUEST_ID
    (trap '' PIPE; printf 'MBX1\t%s\tPROMPT\t%s\t%s\t%s\t%s\n' \
        "$id" "$escaped_cwd" "$status" "$duration_ms" "$flags" \
        >&"$_MBX_ENGINE_IN_FD") || return 1
    IFS=$'\t' read -r -t "${MBX_IPC_TIMEOUT:-0.10}" \
        response_magic response_id response_kind payload extra \
        <&"$_MBX_ENGINE_OUT_FD" || return 1
    [[ $response_magic == MBX1 && $response_id == "$id" && $response_kind == PROMPT && -z $extra ]] || return 1
    _mbx_unescape_field "$payload"
    PS1=$REPLY
}

_mbx_prompt_per_call() {
    local status=$1
    local duration_ms=$2
    local -a args=(prompt --cwd "$PWD" --status "$status")
    if [[ $duration_ms != - ]]; then
        args+=(--duration-ms "$duration_ms")
    fi
    if [[ ! -t 1 || ${TERM:-dumb} == dumb || -n ${NO_COLOR+x} || ${MBX_COLOR:-auto} == never ]]; then
        args+=(--no-color)
    fi
    case ${MBX_ICONS:-auto} in
        never|ascii) args+=(--ascii) ;;
        nerd) args+=(--nerd-font) ;;
    esac
    [[ -z ${SSH_CONNECTION:-}${SSH_TTY:-} ]] || args+=(--ssh)
    [[ ${MBX_PRODUCTION_CONTEXT:-0} != 1 ]] || args+=(--production)
    [[ ${MBX_DISABLE_GIT:-0} != 1 ]] || args+=(--disable-git)
    PS1=$("$MBX_BIN" "${args[@]}" 2>/dev/null)
}

_mbx_update_prompt() {
    local status=$1
    local duration_ms=${2:--}
    local rendered=0

    if [[ ${_MBX_ENGINE_READY:-0} == 1 ]]; then
        if _mbx_prompt_from_coprocess "$status" "$duration_ms"; then
            rendered=1
        else
            _mbx_engine_stop
        fi
    fi

    if (( rendered == 0 )) && [[ ${MBX_DISABLE_RENDERER:-0} != 1 && -x ${MBX_BIN:-} && ${MBX_IPC_MODE:-auto} != off ]]; then
        if _mbx_prompt_per_call "$status" "$duration_ms"; then
            rendered=1
        fi
    fi

    if (( rendered == 0 )); then
        _mbx_fallback_prompt "$status" "$duration_ms"
        PS1=$REPLY
    fi
}
