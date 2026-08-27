# shellcheck shell=bash
# Opt-in syntax highlighting (ADR 0013). Plain bytes live in _MBX_HIGHLIGHT_PLAIN;
# READLINE_LINE holds styled text with Readline non-printing markers. Enter
# restores plain bytes before accept-line. Do not combine with MBX_GHOST=1.

_MBX_HIGHLIGHT_ACCEPT_DEFAULT_KEYSEQ='\C-x\C-m'
_MBX_HIGHLIGHT_PLAIN=
_MBX_HIGHLIGHT_POINT=0
_MBX_HIGHLIGHT_ACTIVE=0
_MBX_HIGHLIGHT_BOUND=0
_MBX_HIGHLIGHT_VI_BOUND=0
_MBX_HIGHLIGHT_ACCEPT_KEYSEQ=$_MBX_HIGHLIGHT_ACCEPT_DEFAULT_KEYSEQ

_mbx_highlight_strip_line() {
    local line=$1
    local plain= byte
    local index=0
    local skipping=0
    local LC_ALL=C

    plain=
    while ((index < ${#line})); do
        byte=${line:index:1}
        printf -v code '%d' "'$byte"
        if ((skipping)); then
            ((code == 2)) && skipping=0
        elif ((code == 1)); then
            skipping=1
        elif ((code == 27)) && [[ ${line:index+1:1} == '[' ]]; then
            index=$((index + 2))
            while ((index < ${#line})); do
                byte=${line:index:1}
                [[ $byte == m ]] && break
                index=$((index + 1))
            done
        else
            plain+=$byte
        fi
        index=$((index + 1))
    done
    REPLY=$plain
}

_mbx_highlight_read_two_lines() {
    local fd=$1
    local deadline=$2
    local timeout status=0
    local line1= line2=
    local LC_ALL=C

    _mbx_deadline_remaining "$deadline" || return 1
    timeout=$REPLY
    IFS= read -r -t "$timeout" -n 65536 -u "$fd" line1 || status=$?
    case $status in
        0 | 1) ;;
        *) return 1 ;;
    esac
    [[ -n $line1 ]] || return 1
    _mbx_deadline_remaining "$deadline" || return 1
    timeout=$REPLY
    IFS= read -r -t "$timeout" -n 64 -u "$fd" line2 || status=$?
    case $status in
        0 | 1) ;;
        *) return 1 ;;
    esac
    [[ $line2 =~ ^[0-9]+$ ]] || return 1
    REPLY=$line1
    _MBX_HIGHLIGHT_STYLED_POINT=$line2
    return 0
}

_mbx_highlight_refresh() {
    local deadline output_fd child_pid plain=${_MBX_HIGHLIGHT_PLAIN-} point=${_MBX_HIGHLIGHT_POINT:-0}

    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ ${MBX_HIGHLIGHT:-} == 1 ]] || return 1
    _mbx_deadline_after "${MBX_HIGHLIGHT_TIMEOUT:-0.05}" || return 1
    deadline=$REPLY
    exec {output_fd}< <(exec "$MBX_BIN" highlight "$plain" --point "$point" 2>/dev/null)
    child_pid=$!
    if ! _mbx_highlight_read_two_lines "$output_fd" "$deadline"; then
        exec {output_fd}<&-
        _mbx_wait_child_until "$child_pid" "$deadline" >/dev/null || \
            _mbx_terminate_child "$child_pid"
        return 1
    fi
    exec {output_fd}<&-
    local styled_line=$REPLY
    local styled_point=${_MBX_HIGHLIGHT_STYLED_POINT:-0}
    _mbx_wait_child_until "$child_pid" "$deadline" >/dev/null || \
        _mbx_terminate_child "$child_pid"
    _mbx_text_has_c0_or_del "$styled_line" && return 1
    READLINE_LINE=$styled_line
    READLINE_POINT=$styled_point
    _MBX_HIGHLIGHT_ACTIVE=1
    return 0
}

_mbx_highlight_capture_plain() {
    if [[ ${_MBX_HIGHLIGHT_ACTIVE:-0} == 1 ]]; then
        return 0
    fi
    _mbx_highlight_strip_line "${READLINE_LINE-}"
    _MBX_HIGHLIGHT_PLAIN=$REPLY
    _MBX_HIGHLIGHT_POINT=${READLINE_POINT:-0}
}

_mbx_highlight_self_insert() {
    local ch=${1-}
    local plain=${_MBX_HIGHLIGHT_PLAIN-}
    local point=${_MBX_HIGHLIGHT_POINT:-0}

    [[ -n $ch ]] || ch=${READLINE_KEYSEQ-}
    [[ -n $ch ]] || return 0
    _mbx_text_has_c0_or_del "$ch" && return 0
    _mbx_highlight_capture_plain
    plain=${_MBX_HIGHLIGHT_PLAIN-}
    point=${_MBX_HIGHLIGHT_POINT:-0}
    _MBX_HIGHLIGHT_PLAIN=${plain:0:point}${ch}${plain:point}
    _MBX_HIGHLIGHT_POINT=$((point + ${#ch}))
    if ! _mbx_highlight_refresh; then
        READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN}
        READLINE_POINT=${_MBX_HIGHLIGHT_POINT}
        _MBX_HIGHLIGHT_ACTIVE=0
    fi
}

_mbx_highlight_backspace() {
    local plain=${_MBX_HIGHLIGHT_PLAIN-}
    local point=${_MBX_HIGHLIGHT_POINT:-0}

    _mbx_highlight_capture_plain
    plain=${_MBX_HIGHLIGHT_PLAIN-}
    point=${_MBX_HIGHLIGHT_POINT:-0}
    if ((point > 0)); then
        _MBX_HIGHLIGHT_PLAIN=${plain:0:point-1}${plain:point}
        _MBX_HIGHLIGHT_POINT=$((point - 1))
        if ! _mbx_highlight_refresh; then
            READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN}
            READLINE_POINT=${_MBX_HIGHLIGHT_POINT}
            _MBX_HIGHLIGHT_ACTIVE=0
        fi
    fi
}

_mbx_highlight_accept_line() {
    READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
    READLINE_POINT=${_MBX_HIGHLIGHT_POINT:-0}
    _MBX_HIGHLIGHT_ACTIVE=0
    bind "\"${_MBX_HIGHLIGHT_ACCEPT_KEYSEQ}\": accept-line"
    bind "\"${_MBX_HIGHLIGHT_ACCEPT_KEYSEQ}\""
}

_mbx_highlight_keyseq_occupied() {
    local keyseq=$1
    local keymap=$2
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    bind -m "$keymap" -p 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    return 1
}

_mbx_highlight_bind_x() {
    local keymap=$1
    local keyseq=$2
    local command=$3
    local allowed=$4
    local fn
    if _mbx_highlight_keyseq_occupied "$keyseq" "$keymap" && \
        [[ ${MBX_HIGHLIGHT_OVERRIDE:-0} != 1 ]]; then
        return 1
    fi
    if bind -m "$keymap" -X 2>/dev/null | grep -Fq "\"$keyseq\":"; then
        [[ ${MBX_HIGHLIGHT_OVERRIDE:-0} == 1 ]] || return 1
    else
        fn=$(bind -m "$keymap" -p 2>/dev/null | grep -F "\"$keyseq\":" | head -n 1)
        fn=${fn##*: }
        fn=${fn# }
        [[ -z $fn || $fn == "$allowed" ]] || return 1
    fi
    bind -m "$keymap" -x "\"$keyseq\": $command"
}

_mbx_highlight_bind_self_chars() {
    local keymap=$1
    local chars=$2
    local i char wrapped=0 quoted
    for ((i = 0; i < ${#chars}; i++)); do
        char=${chars:i:1}
        printf -v quoted '%q' "$char"
        if _mbx_highlight_bind_x "$keymap" "$char" "_mbx_highlight_self_insert $quoted" \
            self-insert; then
            wrapped=1
        fi
    done
    ((wrapped == 1))
}

_mbx_highlight_install_keymap() {
    local keymap=$1
    local chars=$'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-.:/!#$%&()*+,;<=>?@[]^{|}~\'"`\\'
    local accept_key=${_MBX_HIGHLIGHT_ACCEPT_KEYSEQ}
    local wrapped=0
    _mbx_highlight_bind_self_chars "$keymap" "$chars" && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-h' _mbx_highlight_backspace backward-delete-char \
        && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-?' _mbx_highlight_backspace backward-delete-char \
        && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-m' _mbx_highlight_accept_line accept-line \
        && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-j' _mbx_highlight_accept_line accept-line \
        && wrapped=1
    bind -m "$keymap" "\"$accept_key\": accept-line" 2>/dev/null || true
    ((wrapped == 1))
}

_mbx_highlight_install() {
    [[ ${_MBX_HIGHLIGHT_INSTALLED:-0} != 1 ]] || return 0
    _MBX_HIGHLIGHT_BOUND=0
    _MBX_HIGHLIGHT_VI_BOUND=0
    if [[ $- != *i* || ! -t 0 ]]; then
        _MBX_HIGHLIGHT_INSTALLED=1
        return 0
    fi
    if [[ ${MBX_HIGHLIGHT:-} != 1 ]]; then
        _MBX_HIGHLIGHT_INSTALLED=1
        return 0
    fi
    if [[ ${MBX_GHOST:-} == 1 ]]; then
        _MBX_HIGHLIGHT_INSTALLED=1
        return 0
    fi
    _MBX_HIGHLIGHT_ACCEPT_KEYSEQ=${MBX_HIGHLIGHT_ACCEPT_KEYSEQ:-$_MBX_HIGHLIGHT_ACCEPT_DEFAULT_KEYSEQ}
    if _mbx_highlight_install_keymap emacs; then
        _MBX_HIGHLIGHT_BOUND=1
    fi
    if _mbx_highlight_install_keymap vi-insert; then
        _MBX_HIGHLIGHT_VI_BOUND=1
    fi
    _MBX_HIGHLIGHT_INSTALLED=1
}
