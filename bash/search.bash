# shellcheck shell=bash
# Explicit history-search bind -x (ADR 0009). Replaces READLINE_LINE with one
# sidecar match without executing it. Not after-every-key decoration.

_MBX_SEARCH_DEFAULT_KEYSEQ='\C-x\C-r'

_mbx_search_helper() {
    local deadline output_fd child_pid child_status match=
    local read_ok=0

    [[ -x ${MBX_BIN:-} ]] || return 1
    _mbx_deadline_after "${MBX_SEARCH_TIMEOUT:-${MBX_HISTORY_TIMEOUT:-0.10}}" || \
        return 1
    deadline=$REPLY
    exec {output_fd}< <(exec "$MBX_BIN" "$@" 2>/dev/null)
    child_pid=$!
    if _mbx_read_bounded_response "$output_fd" "$deadline"; then
        match=$REPLY
        read_ok=1
    fi
    exec {output_fd}<&-
    if ! _mbx_wait_child_until "$child_pid" "$deadline"; then
        _mbx_terminate_child "$child_pid"
        return 1
    fi
    child_status=$REPLY
    ((read_ok == 1 && child_status == 0)) || return 1
    REPLY=$match
}

_mbx_search_insert() {
    local query=${READLINE_LINE-}
    local match=

    [[ ${MBX_HISTORY:-} == 1 ]] || return 0
    if [[ -z $query ]]; then
        _mbx_search_helper history search recent --limit 1 || return 0
        match=$REPLY
    else
        _mbx_search_helper history search prefix "$query" --limit 1 || return 0
        match=$REPLY
        if [[ -z $match ]]; then
            _mbx_search_helper history search fuzzy "$query" --limit 1 || return 0
            match=$REPLY
        fi
    fi
    [[ -n $match ]] || return 0
    READLINE_LINE=$match
    READLINE_POINT=${#match}
}

_mbx_search_keyseq_occupied() {
    local keyseq=$1
    local keymap=$2
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    bind -m "$keymap" -p 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    return 1
}

_mbx_search_install_keymap() {
    local keymap=$1
    local keyseq=$2
    if _mbx_search_keyseq_occupied "$keyseq" "$keymap" && \
        [[ ${MBX_SEARCH_OVERRIDE:-0} != 1 ]]; then
        return 1
    fi
    bind -m "$keymap" -x "\"$keyseq\": _mbx_search_insert"
}

_mbx_search_install() {
    [[ ${_MBX_SEARCH_INSTALLED:-0} != 1 ]] || return 0
    if [[ $- != *i* ]]; then
        _MBX_SEARCH_INSTALLED=1
        return 0
    fi
    local keyseq=${MBX_SEARCH_KEYSEQ:-$_MBX_SEARCH_DEFAULT_KEYSEQ}
    _MBX_SEARCH_BOUND=0
    _MBX_SEARCH_VI_INSERT_BOUND=0
    _MBX_SEARCH_KEYSEQ_ACTIVE=$keyseq
    if _mbx_search_install_keymap emacs "$keyseq"; then
        _MBX_SEARCH_BOUND=1
    fi
    if _mbx_search_install_keymap vi-insert "$keyseq"; then
        _MBX_SEARCH_VI_INSERT_BOUND=1
    fi
    _MBX_SEARCH_INSTALLED=1
}
