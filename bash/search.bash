# shellcheck shell=bash
# Explicit history-search bind -x (ADR 0009). Replaces READLINE_LINE with one
# sidecar match without executing it. Not after-every-key decoration.
# Default insert `\C-xh` and restore `\C-xl` are unbound in stock emacs.
# Do not use `\C-x\C-r` (re-read-init-file), `\C-x\C-s` (terminal XOFF / IXON),
# `\C-g` / `\C-x\C-g` (abort), or `\C-r` (reverse-i-search).

_MBX_SEARCH_DEFAULT_KEYSEQ='\C-xh'
_MBX_SEARCH_RESTORE_DEFAULT_KEYSEQ='\C-xl'
_MBX_SEARCH_DEFAULT_LIMIT=8
_MBX_SEARCH_MAX_LIMIT=16
_MBX_SEARCH_MATCHES=()
_MBX_SEARCH_INDEX=0
_MBX_SEARCH_ORIGINAL=
_MBX_SEARCH_ORIGINAL_POINT=0
_MBX_SEARCH_HAS_ORIGINAL=0

_mbx_search_clear() {
    _MBX_SEARCH_MATCHES=()
    _MBX_SEARCH_INDEX=0
    _MBX_SEARCH_ORIGINAL=
    _MBX_SEARCH_ORIGINAL_POINT=0
    _MBX_SEARCH_HAS_ORIGINAL=0
}

_mbx_search_limit() {
    local limit=${MBX_SEARCH_LIMIT:-$_MBX_SEARCH_DEFAULT_LIMIT}
    if [[ ! $limit =~ ^[1-9][0-9]*$ ]]; then
        limit=$_MBX_SEARCH_DEFAULT_LIMIT
    fi
    if ((limit > _MBX_SEARCH_MAX_LIMIT)); then
        limit=$_MBX_SEARCH_MAX_LIMIT
    fi
    REPLY=$limit
}

_mbx_search_read_line() {
    local fd=$1
    local deadline=$2
    local timeout status=0
    local LC_ALL=C

    # Do not use `_mbx_read_bounded_response`: it rejects a pipe that already
    # contains a second LF, which is normal for multi-line CLI search output
    # (M-045).
    _mbx_deadline_remaining "$deadline" || return 1
    timeout=$REPLY
    REPLY=
    IFS= read -r -t "$timeout" -n 65536 -u "$fd" REPLY || status=$?
    case $status in
        0)
            if [[ $REPLY == *$'\r' ]]; then
                REPLY=${REPLY%$'\r'}
            fi
            return 0
            ;;
        1)
            [[ -n $REPLY ]] || return 1
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

_mbx_search_helper() {
    local limit=$1
    shift
    local deadline output_fd child_pid child_status=1

    _MBX_SEARCH_MATCHES=()
    _MBX_SEARCH_INDEX=0
    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ $limit =~ ^[1-9][0-9]*$ ]] || return 1
    _mbx_deadline_after "${MBX_SEARCH_TIMEOUT:-${MBX_HISTORY_TIMEOUT:-0.10}}" || \
        return 1
    deadline=$REPLY
    exec {output_fd}< <(exec "$MBX_BIN" "$@" 2>/dev/null)
    child_pid=$!
    while ((${#_MBX_SEARCH_MATCHES[@]} < limit)); do
        if ! _mbx_search_read_line "$output_fd" "$deadline"; then
            break
        fi
        [[ -n $REPLY ]] || break
        _MBX_SEARCH_MATCHES+=("$REPLY")
    done
    exec {output_fd}<&-
    if ! _mbx_wait_child_until "$child_pid" "$deadline"; then
        _mbx_terminate_child "$child_pid"
    else
        child_status=$REPLY
    fi
    if ((${#_MBX_SEARCH_MATCHES[@]} > 0)); then
        return 0
    fi
    ((child_status == 0))
}

_mbx_search_query() {
    local query=$1
    local limit

    _mbx_search_limit
    limit=$REPLY
    if [[ -z $query ]]; then
        _mbx_search_helper "$limit" history search recent --limit "$limit"
        return
    fi
    _mbx_search_helper "$limit" history search prefix "$query" --limit "$limit" || \
        return 1
    if ((${#_MBX_SEARCH_MATCHES[@]} == 0)); then
        _mbx_search_helper "$limit" history search fuzzy "$query" --limit "$limit" || \
            return 1
    fi
    return 0
}

_mbx_search_apply() {
    local match=${_MBX_SEARCH_MATCHES[_MBX_SEARCH_INDEX]-}
    [[ -n $match ]] || return 0
    READLINE_LINE=$match
    READLINE_POINT=${#match}
}

_mbx_search_insert() {
    local query=${READLINE_LINE-}
    local current=$query
    local original=$query
    local original_point=${READLINE_POINT:-0}
    local count

    [[ ${MBX_HISTORY:-} == 1 ]] || return 0
    count=${#_MBX_SEARCH_MATCHES[@]}
    if ((count > 0)) && \
        [[ $current == "${_MBX_SEARCH_MATCHES[_MBX_SEARCH_INDEX]}" ]]; then
        _MBX_SEARCH_INDEX=$(( (_MBX_SEARCH_INDEX + 1) % count ))
        _mbx_search_apply
        return 0
    fi
    if ! _mbx_search_query "$query" || ((${#_MBX_SEARCH_MATCHES[@]} == 0)); then
        _mbx_search_clear
        return 0
    fi
    _MBX_SEARCH_ORIGINAL=$original
    _MBX_SEARCH_ORIGINAL_POINT=$original_point
    _MBX_SEARCH_HAS_ORIGINAL=1
    _MBX_SEARCH_INDEX=0
    _mbx_search_apply
}

_mbx_search_restore() {
    [[ ${MBX_HISTORY:-} == 1 ]] || return 0
    [[ ${_MBX_SEARCH_HAS_ORIGINAL:-0} == 1 ]] || return 0
    READLINE_LINE=${_MBX_SEARCH_ORIGINAL-}
    READLINE_POINT=${_MBX_SEARCH_ORIGINAL_POINT:-0}
    _mbx_search_clear
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
    local fn=$3
    local override=$4
    if _mbx_search_keyseq_occupied "$keyseq" "$keymap" && \
        [[ $override != 1 ]]; then
        return 1
    fi
    bind -m "$keymap" -x "\"$keyseq\": $fn"
}

_mbx_search_install() {
    [[ ${_MBX_SEARCH_INSTALLED:-0} != 1 ]] || return 0
    if [[ $- != *i* ]]; then
        _MBX_SEARCH_INSTALLED=1
        return 0
    fi
    local keyseq=${MBX_SEARCH_KEYSEQ:-$_MBX_SEARCH_DEFAULT_KEYSEQ}
    local restore_keyseq=${MBX_SEARCH_RESTORE_KEYSEQ:-$_MBX_SEARCH_RESTORE_DEFAULT_KEYSEQ}
    local override=${MBX_SEARCH_OVERRIDE:-0}
    local restore_override=${MBX_SEARCH_RESTORE_OVERRIDE:-0}
    _MBX_SEARCH_BOUND=0
    _MBX_SEARCH_VI_INSERT_BOUND=0
    _MBX_SEARCH_RESTORE_BOUND=0
    _MBX_SEARCH_RESTORE_VI_INSERT_BOUND=0
    _MBX_SEARCH_KEYSEQ_ACTIVE=$keyseq
    _MBX_SEARCH_RESTORE_KEYSEQ_ACTIVE=$restore_keyseq
    if _mbx_search_install_keymap emacs "$keyseq" _mbx_search_insert "$override"; then
        _MBX_SEARCH_BOUND=1
    fi
    if _mbx_search_install_keymap vi-insert "$keyseq" _mbx_search_insert "$override"; then
        _MBX_SEARCH_VI_INSERT_BOUND=1
    fi
    if [[ $restore_keyseq != "$keyseq" ]]; then
        if _mbx_search_install_keymap emacs "$restore_keyseq" _mbx_search_restore "$restore_override"; then
            _MBX_SEARCH_RESTORE_BOUND=1
        fi
        if _mbx_search_install_keymap vi-insert "$restore_keyseq" _mbx_search_restore "$restore_override"; then
            _MBX_SEARCH_RESTORE_VI_INSERT_BOUND=1
        fi
    fi
    _MBX_SEARCH_INSTALLED=1
}
