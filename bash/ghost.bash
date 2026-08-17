# shellcheck shell=bash
# Opt-in inline ghost suffix (ADR 0010). Suggestion lives after READLINE_POINT.
# Enter strips it. It never executes automatically. Not after-every-key paint.

_MBX_GHOST_STRIP_DEFAULT_KEYSEQ='\C-xg'
_MBX_GHOST_HAS=0

_mbx_ghost_clear() {
    _MBX_GHOST_HAS=0
}

_mbx_ghost_strip() {
    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        READLINE_LINE=${READLINE_LINE:0:READLINE_POINT}
        _MBX_GHOST_HAS=0
    fi
}

_mbx_ghost_insert_char() {
    local ch=$1
    local point=${READLINE_POINT:-0}
    local line=${READLINE_LINE-}
    READLINE_LINE=${line:0:point}${ch}${line:point}
    READLINE_POINT=$((point + ${#ch}))
}

_mbx_ghost_read_line() {
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

_mbx_ghost_usable_match() {
    local typed=$1
    local match=$2
    local suffix
    local i c
    local LC_ALL=C

    [[ -n $typed && -n $match ]] || return 1
    [[ $match == "$typed"* ]] || return 1
    suffix=${match#"$typed"}
    [[ -n $suffix ]] || return 1
    ((${#suffix} <= 256)) || return 1
    for ((i = 0; i < ${#suffix}; i++)); do
        c=${suffix:i:1}
        if [[ $c < ' ' || $c == $'\177' ]]; then
            return 1
        fi
    done
    REPLY=$match
}

_mbx_ghost_query() {
    local query=$1
    local deadline output_fd child_pid match=

    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ -n $query ]] || return 1
    _mbx_deadline_after "${MBX_GHOST_TIMEOUT:-${MBX_HISTORY_TIMEOUT:-0.10}}" || \
        return 1
    deadline=$REPLY
    exec {output_fd}< <(exec "$MBX_BIN" history search prefix "$query" --limit 1 \
        2>/dev/null)
    child_pid=$!
    if _mbx_ghost_read_line "$output_fd" "$deadline"; then
        match=$REPLY
    fi
    exec {output_fd}<&-
    if ! _mbx_wait_child_until "$child_pid" "$deadline"; then
        _mbx_terminate_child "$child_pid"
    fi
    _mbx_ghost_usable_match "$query" "$match" || return 1
}

_mbx_ghost_refresh() {
    local typed=${READLINE_LINE-}
    local point=${READLINE_POINT:-0}

    _MBX_GHOST_HAS=0
    [[ ${MBX_GHOST:-} == 1 && ${MBX_HISTORY:-} == 1 ]] || return 0
    ((point == ${#typed})) || return 0
    [[ -n $typed ]] || return 0
    _mbx_ghost_query "$typed" || return 0
    READLINE_LINE=$REPLY
    READLINE_POINT=$point
    _MBX_GHOST_HAS=1
}

_mbx_ghost_self_insert() {
    local ch=${READLINE_KEYSEQ-}
    [[ -n $ch ]] || return 0
    _mbx_ghost_strip
    _mbx_ghost_insert_char "$ch"
    _mbx_ghost_refresh
}

_mbx_ghost_backspace() {
    local point=${READLINE_POINT:-0}
    local line=${READLINE_LINE-}
    _mbx_ghost_strip
    point=${READLINE_POINT:-0}
    line=${READLINE_LINE-}
    if ((point > 0)); then
        READLINE_LINE=${line:0:point-1}${line:point}
        READLINE_POINT=$((point - 1))
    fi
    _mbx_ghost_refresh
}

_mbx_ghost_forward() {
    local point=${READLINE_POINT:-0}
    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        READLINE_POINT=${#READLINE_LINE}
        _MBX_GHOST_HAS=0
        return 0
    fi
    if ((point < ${#READLINE_LINE})); then
        READLINE_POINT=$((point + 1))
    fi
}

_mbx_ghost_keyseq_has_x() {
    local keyseq=$1
    local keymap=$2
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "\"$keyseq\":"
}

_mbx_ghost_stock_fn() {
    local keyseq=$1
    local keymap=$2
    local line=
    line=$(bind -m "$keymap" -p 2>/dev/null | grep -F "\"$keyseq\":" | head -n 1) || \
        line=
    REPLY=${line##*: }
    REPLY=${REPLY# }
}

_mbx_ghost_can_wrap() {
    local keyseq=$1
    local keymap=$2
    local allowed=$3
    local fn
    if _mbx_ghost_keyseq_has_x "$keyseq" "$keymap"; then
        [[ ${MBX_GHOST_OVERRIDE:-0} == 1 ]] || return 1
        return 0
    fi
    _mbx_ghost_stock_fn "$keyseq" "$keymap"
    fn=$REPLY
    [[ -z $fn || $fn == "$allowed" ]]
}

_mbx_ghost_bind_x() {
    local keymap=$1
    local keyseq=$2
    local fn=$3
    local allowed=$4
    _mbx_ghost_can_wrap "$keyseq" "$keymap" "$allowed" || return 1
    bind -m "$keymap" -x "\"$keyseq\": $fn"
}

_mbx_ghost_bind_self_chars() {
    local keymap=$1
    local chars=$2
    local i char wrapped=0
    for ((i = 0; i < ${#chars}; i++)); do
        char=${chars:i:1}
        if _mbx_ghost_bind_x "$keymap" "$char" _mbx_ghost_self_insert self-insert; then
            wrapped=1
        fi
    done
    ((wrapped == 1))
}

_mbx_ghost_install() {
    [[ ${_MBX_GHOST_INSTALLED:-0} != 1 ]] || return 0
    _MBX_GHOST_BOUND=0
    if [[ $- != *i* ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    if [[ ${MBX_GHOST:-} != 1 || ${MBX_HISTORY:-} != 1 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    local strip=${MBX_GHOST_STRIP_KEYSEQ:-$_MBX_GHOST_STRIP_DEFAULT_KEYSEQ}
    local chars='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-.:/=+,[]@~'
    if _mbx_ghost_keyseq_has_x "$strip" emacs && \
        [[ ${MBX_GHOST_OVERRIDE:-0} != 1 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    _mbx_ghost_stock_fn "$strip" emacs
    if [[ -n $REPLY && ${MBX_GHOST_OVERRIDE:-0} != 1 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    _mbx_ghost_bind_self_chars emacs "$chars" || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_bind_x emacs '\C-h' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x emacs '\C-?' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x emacs '\e[C' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x emacs '\C-f' _mbx_ghost_forward forward-char || true
    bind -m emacs -x "\"$strip\": _mbx_ghost_strip"
    _mbx_ghost_stock_fn '\C-m' emacs
    if [[ $REPLY == accept-line || -z $REPLY || ${MBX_GHOST_OVERRIDE:-0} == 1 ]]; then
        bind -m emacs "\"\\C-m\": \"${strip}\\C-j\""
        _MBX_GHOST_BOUND=1
    fi
    _MBX_GHOST_INSTALLED=1
}
