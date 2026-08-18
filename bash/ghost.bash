# shellcheck shell=bash
# Opt-in inline ghost suffix (ADR 0010). Suggestion lives after READLINE_POINT.
# `\C-xg` is glob-list-expansions (M-040). Enter is not bind -x: a -x step in a
# keyseq macro drops the remaining keys (M-041), so an active suffix arms a
# Readline-only kill-line + accept-line macro on `\C-m` and `\C-j` instead.
# Cycle next/prev default to unbound `\C-x\C-n` / `\C-x\C-p` (not `\en`/`\ep`).
# Remaining ASCII printables use Readline quoted keyseqs so `"` / `\` bind.
_MBX_GHOST_KILL_DEFAULT_KEYSEQ='\C-x\C-k'
_MBX_GHOST_ACCEPT_DEFAULT_KEYSEQ='\C-x\C-m'
_MBX_GHOST_NEXT_DEFAULT_KEYSEQ='\C-x\C-n'
_MBX_GHOST_PREV_DEFAULT_KEYSEQ='\C-x\C-p'
_MBX_GHOST_HAS=0
_MBX_GHOST_POINT=0
_MBX_GHOST_BOUND=0
_MBX_GHOST_CYCLE_BOUND=0
_MBX_GHOST_ENTER_ARMED=0
_MBX_GHOST_WRAP_CTRL_J=0
_MBX_GHOST_INDEX=0
_MBX_GHOST_TYPED_LEN=0
_MBX_GHOST_CANDIDATES=()
_MBX_GHOST_KILL_KEYSEQ=$_MBX_GHOST_KILL_DEFAULT_KEYSEQ
_MBX_GHOST_ACCEPT_KEYSEQ=$_MBX_GHOST_ACCEPT_DEFAULT_KEYSEQ

_mbx_ghost_disarm_enter() {
    [[ ${_MBX_GHOST_ENTER_ARMED:-0} == 1 ]] || return 0
    bind -m emacs '"\C-m": accept-line' || return 1
    if [[ ${_MBX_GHOST_WRAP_CTRL_J:-0} == 1 ]]; then
        bind -m emacs '"\C-j": accept-line' || return 1
    fi
    _MBX_GHOST_ENTER_ARMED=0
}

_mbx_ghost_arm_enter() {
    local macro
    [[ ${_MBX_GHOST_BOUND:-0} == 1 ]] || return 0
    [[ ${_MBX_GHOST_ENTER_ARMED:-0} == 1 ]] && return 0
    macro="${_MBX_GHOST_KILL_KEYSEQ}${_MBX_GHOST_ACCEPT_KEYSEQ}"
    bind -m emacs "\"\\C-m\": \"$macro\"" || return 1
    if [[ ${_MBX_GHOST_WRAP_CTRL_J:-0} == 1 ]]; then
        if ! bind -m emacs "\"\\C-j\": \"$macro\""; then
            bind -m emacs '"\C-m": accept-line' || true
            return 1
        fi
    fi
    _MBX_GHOST_ENTER_ARMED=1
}

_mbx_ghost_reset_state() {
    _MBX_GHOST_HAS=0
    _MBX_GHOST_POINT=0
    _MBX_GHOST_INDEX=0
    _MBX_GHOST_TYPED_LEN=0
    _MBX_GHOST_CANDIDATES=()
}

_mbx_ghost_clear() {
    _mbx_ghost_reset_state
    _mbx_ghost_disarm_enter || true
}

_mbx_ghost_strip() {
    local cut
    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        cut=${_MBX_GHOST_POINT:-0}
        READLINE_LINE=${READLINE_LINE:0:cut}
        READLINE_POINT=$cut
    fi
    _mbx_ghost_reset_state
    _mbx_ghost_disarm_enter || true
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

_mbx_ghost_limit() {
    local n=${MBX_GHOST_LIMIT:-8}
    case $n in
        '' | *[!0-9]*)
            n=8
            ;;
    esac
    if ((10#$n < 1)); then
        n=1
    fi
    if ((10#$n > 8)); then
        n=8
    fi
    REPLY=$n
}

_mbx_ghost_candidate_seen() {
    local candidate=$1
    local existing
    for existing in "${_MBX_GHOST_CANDIDATES[@]}"; do
        [[ $existing == "$candidate" ]] && return 0
    done
    return 1
}

_mbx_ghost_query() {
    local query=$1
    local deadline output_fd child_pid match= limit
    local monitor=0 notify=0

    _MBX_GHOST_CANDIDATES=()
    _MBX_GHOST_INDEX=0
    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ -n $query ]] || return 1
    _mbx_ghost_limit
    limit=$REPLY
    _mbx_deadline_after "${MBX_GHOST_TIMEOUT:-${MBX_HISTORY_TIMEOUT:-0.10}}" || \
        return 1
    deadline=$REPLY
    [[ $- == *m* ]] && monitor=1
    [[ $- == *b* ]] && notify=1
    set +m
    set +b
    exec {output_fd}< <(exec "$MBX_BIN" history search prefix "$query" --limit "$limit" \
        2>/dev/null) || {
        ((notify == 1)) && set -b
        ((monitor == 1)) && set -m
        return 1
    }
    child_pid=$!
    while ((${#_MBX_GHOST_CANDIDATES[@]} < limit)); do
        _mbx_ghost_read_line "$output_fd" "$deadline" || break
        match=$REPLY
        _mbx_ghost_usable_match "$query" "$match" || continue
        match=$REPLY
        _mbx_ghost_candidate_seen "$match" && continue
        _MBX_GHOST_CANDIDATES+=("$match")
    done
    exec {output_fd}<&-
    if ! _mbx_wait_child_until "$child_pid" "$deadline"; then
        _mbx_terminate_child "$child_pid"
    fi
    ((notify == 1)) && set -b
    ((monitor == 1)) && set -m
    ((${#_MBX_GHOST_CANDIDATES[@]} > 0)) || return 1
    REPLY=${_MBX_GHOST_CANDIDATES[0]}
}

_mbx_ghost_show() {
    local match=$1
    local typed=$2
    local point=${_MBX_GHOST_TYPED_LEN:-0}

    READLINE_LINE=$match
    READLINE_POINT=$point
    _MBX_GHOST_HAS=1
    _MBX_GHOST_POINT=$point
    if ! _mbx_ghost_arm_enter; then
        READLINE_LINE=$typed
        READLINE_POINT=$point
        _mbx_ghost_reset_state
        return 1
    fi
}

_mbx_ghost_refresh() {
    local typed=${READLINE_LINE-}
    local point=${READLINE_POINT:-0}

    _mbx_ghost_reset_state
    _mbx_ghost_disarm_enter || true
    [[ ${MBX_GHOST:-} == 1 && ${MBX_HISTORY:-} == 1 ]] || return 0
    ((point == ${#typed})) || return 0
    [[ -n $typed ]] || return 0
    _mbx_ghost_query "$typed" || return 0
    _MBX_GHOST_TYPED_LEN=$point
    _mbx_ghost_show "$REPLY" "$typed" || return 0
}

_mbx_ghost_cycle() {
    local delta=${1:-1}
    local n=${#_MBX_GHOST_CANDIDATES[@]}
    local i typed typed_len

    [[ ${_MBX_GHOST_HAS:-0} == 1 ]] || return 0
    ((n >= 2)) || return 0
    typed_len=${_MBX_GHOST_TYPED_LEN:-0}
    typed=${READLINE_LINE:0:typed_len}
    i=$((_MBX_GHOST_INDEX + delta))
    if ((i < 0)); then
        i=$((n - 1))
    fi
    if ((i >= n)); then
        i=0
    fi
    _MBX_GHOST_INDEX=$i
    _mbx_ghost_show "${_MBX_GHOST_CANDIDATES[i]}" "$typed" || return 0
}

_mbx_ghost_cycle_next() {
    _mbx_ghost_cycle 1
}

_mbx_ghost_cycle_prev() {
    _mbx_ghost_cycle -1
}

_mbx_ghost_self_insert() {
    local ch=${1-}
    [[ -n $ch ]] || ch=${READLINE_KEYSEQ-}
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
        _mbx_ghost_reset_state
        _mbx_ghost_disarm_enter || true
        return 0
    fi
    if ((point < ${#READLINE_LINE})); then
        READLINE_POINT=$((point + 1))
    fi
}

_mbx_ghost_forward_word() {
    local line=${READLINE_LINE-}
    local point=${READLINE_POINT:-0}
    local n=${#line}
    local c
    local LC_ALL=C

    while ((point < n)); do
        c=${line:point:1}
        [[ $c == [[:alnum:]] ]] && break
        point=$((point + 1))
    done
    while ((point < n)); do
        c=${line:point:1}
        [[ $c == [[:alnum:]] ]] || break
        point=$((point + 1))
    done
    READLINE_POINT=$point
    if [[ ${_MBX_GHOST_HAS:-0} != 1 ]]; then
        return 0
    fi
    if ((point >= n)); then
        _mbx_ghost_reset_state
        _mbx_ghost_disarm_enter || true
        return 0
    fi
    _MBX_GHOST_POINT=$point
}

_mbx_ghost_quoted_keyseq() {
    local ch=$1
    local LC_ALL=C
    case $ch in
        \\) REPLY='"\\"' ;;
        \") REPLY='"\""' ;;
        *) REPLY="\"${ch}\"" ;;
    esac
}

_mbx_ghost_keyseq_has_x() {
    local keyseq=$1
    local keymap=$2
    local quoted
    _mbx_ghost_quoted_keyseq "$keyseq"
    quoted=$REPLY
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "${quoted}:"
}

_mbx_ghost_stock_fn() {
    local keyseq=$1
    local keymap=$2
    local quoted line=
    _mbx_ghost_quoted_keyseq "$keyseq"
    quoted=$REPLY
    line=$(bind -m "$keymap" -p 2>/dev/null | grep -F "${quoted}:" | head -n 1) || \
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
    local command=$3
    local allowed=$4
    local spec
    _mbx_ghost_can_wrap "$keyseq" "$keymap" "$allowed" || return 1
    _mbx_ghost_quoted_keyseq "$keyseq"
    spec="${REPLY}: $command"
    bind -m "$keymap" -x "$spec"
}

_mbx_ghost_bind_fn() {
    local keymap=$1
    local keyseq=$2
    local fn=$3
    local allowed=$4
    local spec
    _mbx_ghost_can_wrap "$keyseq" "$keymap" "$allowed" || return 1
    _mbx_ghost_quoted_keyseq "$keyseq"
    spec="${REPLY}: $fn"
    bind -m "$keymap" "$spec"
}

_mbx_ghost_bind_self_chars() {
    local keymap=$1
    local chars=$2
    local i char wrapped=0 quoted
    for ((i = 0; i < ${#chars}; i++)); do
        char=${chars:i:1}
        printf -v quoted '%q' "$char"
        if _mbx_ghost_bind_x "$keymap" "$char" "_mbx_ghost_self_insert $quoted" self-insert; then
            wrapped=1
        fi
    done
    ((wrapped == 1))
}

_mbx_ghost_install() {
    [[ ${_MBX_GHOST_INSTALLED:-0} != 1 ]] || return 0
    _MBX_GHOST_BOUND=0
    _MBX_GHOST_CYCLE_BOUND=0
    _MBX_GHOST_ENTER_ARMED=0
    _MBX_GHOST_WRAP_CTRL_J=0
    if [[ $- != *i* || ! -t 0 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    if [[ ${MBX_GHOST:-} != 1 || ${MBX_HISTORY:-} != 1 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    local kill_key=${MBX_GHOST_KILL_KEYSEQ:-$_MBX_GHOST_KILL_DEFAULT_KEYSEQ}
    local accept_key=${MBX_GHOST_ACCEPT_KEYSEQ:-$_MBX_GHOST_ACCEPT_DEFAULT_KEYSEQ}
    local next_key=${MBX_GHOST_NEXT_KEYSEQ:-$_MBX_GHOST_NEXT_DEFAULT_KEYSEQ}
    local prev_key=${MBX_GHOST_PREV_KEYSEQ:-$_MBX_GHOST_PREV_DEFAULT_KEYSEQ}
    local chars=$'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-.:/!#$%&()*+,;<=>?@[]^{|}~\'"`\\'
    [[ -n $kill_key && -n $accept_key ]] || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_can_wrap "$kill_key" emacs kill-line || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_can_wrap "$accept_key" emacs accept-line || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_stock_fn '\C-m' emacs
    if [[ $REPLY != accept-line ]] || _mbx_ghost_keyseq_has_x '\C-m' emacs; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    _mbx_ghost_stock_fn '\C-j' emacs
    if [[ $REPLY == accept-line ]] && ! _mbx_ghost_keyseq_has_x '\C-j' emacs; then
        _MBX_GHOST_WRAP_CTRL_J=1
    fi
    if [[ ${_MBX_GHOST_WRAP_CTRL_J:-0} != 1 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    if [[ $kill_key == "$accept_key" ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    _mbx_ghost_bind_self_chars emacs "$chars" || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_bind_fn emacs "$kill_key" kill-line kill-line || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_bind_fn emacs "$accept_key" accept-line accept-line || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _MBX_GHOST_KILL_KEYSEQ=$kill_key
    _MBX_GHOST_ACCEPT_KEYSEQ=$accept_key
    _mbx_ghost_bind_x emacs '\C-h' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x emacs '\C-?' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x emacs '\e[C' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x emacs '\C-f' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x emacs '\ef' _mbx_ghost_forward_word forward-word || true
    _mbx_ghost_bind_x emacs '\e[1;5C' _mbx_ghost_forward_word forward-word || true
    _mbx_ghost_bind_x emacs '\e[5C' _mbx_ghost_forward_word forward-word || true
    if [[ -n $next_key && $next_key != "$kill_key" && $next_key != "$accept_key" ]]; then
        if _mbx_ghost_bind_x emacs "$next_key" _mbx_ghost_cycle_next ''; then
            _MBX_GHOST_CYCLE_BOUND=1
        fi
    fi
    if [[ -n $prev_key && $prev_key != "$kill_key" && $prev_key != "$accept_key" && $prev_key != "$next_key" ]]; then
        _mbx_ghost_bind_x emacs "$prev_key" _mbx_ghost_cycle_prev '' || true
    fi
    _MBX_GHOST_BOUND=1
    _MBX_GHOST_INSTALLED=1
}
