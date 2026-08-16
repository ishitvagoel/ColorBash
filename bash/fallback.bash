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

_mbx_role_sgr() {
    local role=$1
    local flags=$2

    if (( flags & _MBX_FLAG_TRUECOLOR )); then
        case $role in
            primary) REPLY='1;38;2;135;175;215' ;;
            path) REPLY='1;38;2;135;215;255' ;;
            repository_clean) REPLY='38;2;135;215;135' ;;
            repository_dirty|warning) REPLY='1;38;2;255;215;135' ;;
            danger) REPLY='1;38;2;255;0;0' ;;
            error) REPLY='1;38;2;255;135;135' ;;
            muted) REPLY='38;2;138;138;138' ;;
            *) return 2 ;;
        esac
    elif (( flags & _MBX_FLAG_COLOR_16 )); then
        case $role in
            primary|path) REPLY='1;36' ;;
            repository_clean) REPLY='1;32' ;;
            repository_dirty|warning) REPLY='1;33' ;;
            danger|error) REPLY='1;31' ;;
            muted) REPLY='1;30' ;;
            *) return 2 ;;
        esac
    else
        case $role in
            primary) REPLY='1;38;5;81' ;;
            path) REPLY='1;38;5;117' ;;
            repository_clean) REPLY='38;5;114' ;;
            repository_dirty|warning) REPLY='1;38;5;215' ;;
            danger) REPLY='1;38;5;196' ;;
            error) REPLY='1;38;5;203' ;;
            muted) REPLY='38;5;245' ;;
            *) return 2 ;;
        esac
    fi
}

_mbx_fallback_prompt() {
    (($# == 4)) || return 2

    local status=$1
    local duration_ms=$2
    local cwd=$3
    local flags=$4
    local path=$cwd
    local context=
    local context_sgr=
    local path_sgr=
    local error_sgr=
    local muted_sgr=
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
        _mbx_role_sgr danger "$flags" || return 2
        context_sgr=$REPLY
    elif ((flags & _MBX_FLAG_SSH)); then
        _mbx_sanitize_text "${HOSTNAME:-remote}"
        host=$REPLY
        context="ssh: ${host}"
        _mbx_role_sgr warning "$flags" || return 2
        context_sgr=$REPLY
    fi

    if (( (flags & _MBX_FLAG_NO_COLOR) == 0 )); then
        if [[ -n $context ]]; then
            first_line="\\[\\e[${context_sgr}m\\]${context}\\[\\e[0m\\]  "
        fi
        _mbx_role_sgr path "$flags" || return 2
        path_sgr=$REPLY
        first_line+="\\[\\e[${path_sgr}m\\]${path}\\[\\e[0m\\]"
        if (( status != 0 )); then
            _mbx_role_sgr error "$flags" || return 2
            error_sgr=$REPLY
            first_line+="  \\[\\e[${error_sgr}m\\]exit ${status}\\[\\e[0m\\]"
        fi
        if [[ $duration_ms != - ]] && (( duration_ms >= 2000 )); then
            _mbx_role_sgr muted "$flags" || return 2
            muted_sgr=$REPLY
            first_line+="  \\[\\e[${muted_sgr}m\\]$((duration_ms / 1000))s\\[\\e[0m\\]"
        fi
        _mbx_role_sgr primary "$flags" || return 2
        arrow="\\[\\e[${REPLY}m\\]>\\[\\e[0m\\]"
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
