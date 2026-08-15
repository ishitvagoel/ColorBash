# shellcheck shell=bash
# Minimal prompt used when the native helper is absent or unhealthy.

_mbx_sanitize_text() {
    local value=${1-}
    value=${value//$'\e'/?}
    value=${value//$'\n'/?}
    value=${value//$'\r'/?}
    value=${value//$'\t'/ }
    value=${value//\\/?}
    value=${value//\$/?}
    value=${value//\`/?}
    REPLY=${value:0:256}
}

_mbx_color_enabled() {
    [[ -t 1 ]] || return 1
    [[ ${TERM:-dumb} != dumb ]] || return 1
    [[ -z ${NO_COLOR+x} ]] || return 1
    [[ ${MBX_COLOR:-auto} != never ]]
}

_mbx_fallback_prompt() {
    local status=${1:-0}
    local duration_ms=${2:--}
    local path=${PWD/#"${HOME:-}"/~}
    local branch=
    local first_line=
    local arrow='>'

    _mbx_sanitize_text "$path"
    path=$REPLY

    if [[ ${MBX_DISABLE_GIT:-0} != 1 ]] && command -v git >/dev/null 2>&1; then
        branch=$(command git -C "$PWD" symbolic-ref --quiet --short HEAD 2>/dev/null) || branch=
        if [[ -n $branch ]]; then
            _mbx_sanitize_text "$branch"
            branch="  git:$REPLY"
        fi
    fi

    if _mbx_color_enabled; then
        first_line="\\[\\e[1;38;5;117m\\]${path}\\[\\e[0m\\]"
        if [[ -n $branch ]]; then
            first_line+="\\[\\e[38;5;114m\\]${branch}\\[\\e[0m\\]"
        fi
        if (( status != 0 )); then
            first_line+="  \\[\\e[1;38;5;203m\\]exit ${status}\\[\\e[0m\\]"
        fi
        if [[ $duration_ms != - ]] && (( duration_ms >= 2000 )); then
            first_line+="  \\[\\e[38;5;245m\\]$((duration_ms / 1000))s\\[\\e[0m\\]"
        fi
        arrow='\[\e[1;38;5;81m\]>\[\e[0m\]'
    else
        first_line="${path}${branch}"
        if (( status != 0 )); then
            first_line+="  exit ${status}"
        fi
        if [[ $duration_ms != - ]] && (( duration_ms >= 2000 )); then
            first_line+="  $((duration_ms / 1000))s"
        fi
    fi

    REPLY="${first_line}\\n${arrow} "
}

