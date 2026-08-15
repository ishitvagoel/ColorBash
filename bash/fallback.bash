# shellcheck shell=bash
# Minimal prompt used when the native helper is absent or unhealthy.

_mbx_sanitize_text() {
    local value=${1-}
    local sanitized= byte
    local code index
    local LC_ALL=C

    # PS1 is later interpreted by Bash. Bound both the work and result while
    # replacing terminal controls and the characters that enable another round
    # of prompt expansion. Bash variables cannot contain NUL; every other C0
    # byte and DEL is handled here.
    for ((index = 0; index < ${#value} && index < 256; index++)); do
        byte=${value:index:1}
        printf -v code '%d' "'$byte"
        if ((code < 32 || code == 127)) || [[ $byte == '$' || $byte == \` || $byte == \\ ]]; then
            sanitized+='?'
        else
            sanitized+=$byte
        fi
    done
    REPLY=$sanitized
}

_mbx_fallback_prompt() {
    (($# == 4)) || return 2

    local status=$1
    local duration_ms=$2
    local cwd=$3
    local flags=$4
    local path=$cwd
    local context=
    local context_color=
    local first_line=
    local host user
    local arrow='>'

    if [[ -n ${HOME:-} ]]; then
        if [[ $cwd == "$HOME" ]]; then
            path='~'
        elif [[ $cwd == "$HOME/"* ]]; then
            path="~${cwd:${#HOME}}"
        fi
    fi

    _mbx_sanitize_text "$path"
    path=$REPLY

    if ((flags & _MBX_FLAG_PRODUCTION)); then
        _mbx_sanitize_text "${HOSTNAME:-host}"
        host=$REPLY
        _mbx_sanitize_text "${USER:-user}"
        user=$REPLY
        context="! PROD · ${host} · ${user}"
        context_color='1;38;5;196'
    elif ((flags & _MBX_FLAG_SSH)); then
        _mbx_sanitize_text "${HOSTNAME:-remote}"
        host=$REPLY
        context="ssh: ${host}"
        context_color='1;38;5;215'
    fi

    if (( (flags & _MBX_FLAG_NO_COLOR) == 0 )); then
        if [[ -n $context ]]; then
            first_line="\\[\\e[${context_color}m\\]${context}\\[\\e[0m\\]  "
        fi
        first_line+="\\[\\e[1;38;5;117m\\]${path}\\[\\e[0m\\]"
        if (( status != 0 )); then
            first_line+="  \\[\\e[1;38;5;203m\\]exit ${status}\\[\\e[0m\\]"
        fi
        if [[ $duration_ms != - ]] && (( duration_ms >= 2000 )); then
            first_line+="  \\[\\e[38;5;245m\\]$((duration_ms / 1000))s\\[\\e[0m\\]"
        fi
        arrow='\[\e[1;38;5;81m\]>\[\e[0m\]'
    else
        if [[ -n $context ]]; then
            first_line="${context}  "
        fi
        first_line+="${path}"
        if (( status != 0 )); then
            first_line+="  exit ${status}"
        fi
        if [[ $duration_ms != - ]] && (( duration_ms >= 2000 )); then
            first_line+="  $((duration_ms / 1000))s"
        fi
    fi

    REPLY="${first_line}\\n${arrow} "
}
