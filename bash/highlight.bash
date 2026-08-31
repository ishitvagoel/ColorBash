# shellcheck shell=bash
# Opt-in syntax highlighting (ADR 0013, ADR 0015). READLINE_LINE stays plain;
# the helper's styled copy is painted on one reserved row below the prompt.
# Do not combine with MBX_GHOST=1.

_MBX_HIGHLIGHT_PLAIN=
_MBX_HIGHLIGHT_POINT=0
_MBX_HIGHLIGHT_BOUND=0
_MBX_HIGHLIGHT_VI_BOUND=0
_MBX_HIGHLIGHT_PAINTED=0
_MBX_HIGHLIGHT_GENERATION=0

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
            local width=1
            if ((code >= 0xF0)); then
                width=4
            elif ((code >= 0xE0)); then
                width=3
            elif ((code >= 0xC0)); then
                width=2
            fi
            plain+=${line:index:width}
            index=$((index + width))
            continue
        fi
        index=$((index + 1))
    done
    REPLY=$plain
}

# Drop SOH/STX only; keep CSI so the preview row can use real SGR (ADR 0015).
_mbx_highlight_strip_markers() {
    local line=$1
    local out= index=0 byte code
    local LC_ALL=C

    while ((index < ${#line})); do
        byte=${line:index:1}
        printf -v code '%d' "'$byte"
        if ((code == 1 || code == 2)); then
            index=$((index + 1))
            continue
        fi
        out+=$byte
        index=$((index + 1))
    done
    REPLY=$out
}

_mbx_highlight_restore_jobs() {
    _mbx_jobs_restore
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

_mbx_highlight_validate_styled() {
    local styled=$1
    local plain_expected=${_MBX_HIGHLIGHT_PLAIN-}
    _mbx_highlight_strip_line "$styled"
    [[ $REPLY == "$plain_expected" ]] || return 1
    _mbx_text_has_c0_or_del "$REPLY" && return 1
    return 0
}

_mbx_highlight_unpaint() {
    [[ ${_MBX_HIGHLIGHT_PAINTED:-0} == 1 ]] || return 0
    if [[ -w /dev/tty ]]; then
        _mbx_tty_erase_below
    fi
    _MBX_HIGHLIGHT_PAINTED=0
}

_mbx_highlight_have_tty() {
    # Same gate as the overlay: stdout must be a tty (module tests are not)
    # and /dev/tty must be writable. The overlay owns the rows below the
    # prompt while it is visible.
    [[ -t 1 && -w /dev/tty ]] || return 1
    [[ ${_MBX_COMP_OVERLAY_VISIBLE:-0} == 1 ]] && return 1
    return 0
}

# ESC (CSI) is required for SGR on the preview row. Every other C0/DEL is
# injection. Do not use a glob range to skip ESC: `$'\030'-$'\037'` is octal
# 24-31 and *includes* ESC (octal 033).
_mbx_highlight_preview_row_ok() {
    local row=$1
    local LC_ALL=C index=0 ch code

    while ((index < ${#row})); do
        ch=${row:index:1}
        printf -v code '%d' "'$ch"
        if ((code == 27)); then
            index=$((index + 1))
            continue
        fi
        if ((code < 32 || code == 127)); then
            return 1
        fi
        index=$((index + 1))
    done
    return 0
}

_mbx_highlight_paint() {
    local styled=$1
    local row

    _mbx_highlight_unpaint
    _mbx_highlight_have_tty || return 0
    _mbx_highlight_strip_markers "$styled"
    row=$REPLY
    _mbx_highlight_preview_row_ok "$row" || return 0
    _mbx_tty_columns
    _mbx_tty_clamp_row "$row" "$REPLY"
    row=$REPLY
    [[ -n $row ]] || return 0
    _mbx_tty_reserve_rows 1
    _mbx_tty_save_cursor
    # Same draw as the overlay (M-065): IND reserved the row, DECSC saved the
    # prompt cell, `\n` steps onto the reserved row without CUP. `\e[K` clears
    # leftover cells so a shorter refresh cannot leave a stale tail.
    printf '\n%s\e[K' "$row" >/dev/tty 2>/dev/null || true
    _mbx_tty_restore_cursor
    _MBX_HIGHLIGHT_PAINTED=1
}

_mbx_highlight_fallback_plain() {
    READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
    READLINE_POINT=${_MBX_HIGHLIGHT_POINT:-0}
    _mbx_highlight_unpaint
}

_mbx_highlight_color_flag() {
    # Paint goes to /dev/tty, not Bash's stdout. `bind -x` widgets often have
    # stdout as a pipe, so `_mbx_color_capable`'s `-t 1` check would keep
    # live color off (the same class of bug as M-062, different side of the
    # boundary). Honor TERM/NO_COLOR/MBX_COLOR, then the controlling tty.
    if [[ ${TERM:-dumb} == dumb || -n ${NO_COLOR+x} || ${MBX_COLOR:-auto} == never ]]; then
        REPLY=0
    elif [[ -t 1 || -t 0 || -w /dev/tty ]]; then
        REPLY=1
    else
        REPLY=0
    fi
}

# Spawn fallback: one helper process per call. Used only when no coprocess is
# attached (MBX_IPC_MODE=off/per-call, or the coprocess died this cycle).
_mbx_highlight_refresh_cli() {
    (($# == 3)) || return 2

    local plain=$1 point=$2 deadline=$3
    local output_fd child_pid color styled_line styled_point

    _mbx_highlight_color_flag
    color=$REPLY
    exec {output_fd}< <(exec "$MBX_BIN" highlight "$plain" --point "$point" \
        --color "$color" 2>/dev/null)
    child_pid=$!
    if ! _mbx_highlight_read_two_lines "$output_fd" "$deadline"; then
        exec {output_fd}<&-
        _mbx_wait_or_kill_child "$child_pid" "$deadline" || true
        return 1
    fi
    styled_line=$REPLY
    styled_point=${_MBX_HIGHLIGHT_STYLED_POINT:-0}
    exec {output_fd}<&-
    if ! _mbx_wait_or_kill_child "$child_pid" "$deadline"; then
        return 1
    fi
    ((REPLY == 0)) || return 1
    REPLY=$styled_line
    _MBX_HIGHLIGHT_STYLED_POINT=$styled_point
    return 0
}

# Coprocess path: one HIGHLIGHT/STYLED round trip (ADR 0014). Color is a
# real `_mbx_highlight_color_flag` decision (ADR 0015); styled bytes are
# painted below the prompt, never assigned to READLINE_LINE (M-064).
_mbx_highlight_refresh_wire() {
    (($# == 4)) || return 2

    local plain=$1 point=$2 generation=$3 deadline=$4
    local request_id response result_gen color
    local -a fields=()

    _mbx_highlight_color_flag
    color=$REPLY
    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID
    _mbx_protocol_encode_highlight "$request_id" "$generation" "$color" "$point" "$plain" || \
        return 1
    if ! _mbx_engine_write "$REPLY" "$deadline"; then
        _mbx_engine_stop
        return 1
    fi
    while _mbx_deadline_remaining "$deadline" >/dev/null; do
        if ! _mbx_engine_read_line "$_MBX_ENGINE_OUT_FD" "$deadline"; then
            return 1
        fi
        response=$REPLY
        if _mbx_protocol_parse_highlight_styled "$response"; then
            result_gen=$REPLY
            if ((result_gen == generation)); then
                REPLY=$_MBX_PROTOCOL_STYLED_LINE
                _MBX_HIGHLIGHT_STYLED_POINT=$_MBX_PROTOCOL_STYLED_POINT
                return 0
            fi
            if ((result_gen < generation)); then
                continue
            fi
            return 1
        fi
        fields=()
        _mbx_protocol_split_fields "$response" fields || {
            _mbx_engine_stop
            return 1
        }
        if ((${#fields[@]} == 3)) && \
            [[ ${fields[0]} == "$_MBX_PROTOCOL_MAGIC_HISTORY" && \
                ${fields[2]} == ACK ]]; then
            continue
        fi
        _mbx_engine_stop
        return 1
    done
    return 1
}

_mbx_highlight_refresh() {
    local deadline plain=${_MBX_HIGHLIGHT_PLAIN-} point=${_MBX_HIGHLIGHT_POINT:-0}
    local styled_line status=1

    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ ${MBX_HIGHLIGHT:-} == 1 ]] || return 1
    _mbx_deadline_after "${MBX_HIGHLIGHT_TIMEOUT:-0.05}" || return 1
    deadline=$REPLY
    _mbx_jobs_suspend
    ((_MBX_HIGHLIGHT_GENERATION += 1))
    if [[ ${_MBX_ENGINE_READY:-0} == 1 ]] && \
        declare -F _mbx_engine_write >/dev/null 2>&1; then
        _mbx_highlight_refresh_wire "$plain" "$point" "$_MBX_HIGHLIGHT_GENERATION" \
            "$deadline" && status=0
    else
        _mbx_highlight_refresh_cli "$plain" "$point" "$deadline" && status=0
    fi
    _mbx_highlight_restore_jobs
    ((status == 0)) || return 1
    styled_line=$REPLY
    _mbx_highlight_validate_styled "$styled_line" || return 1
    READLINE_LINE=$plain
    READLINE_POINT=$point
    _mbx_highlight_paint "$styled_line"
    return 0
}

_mbx_highlight_sync_plain() {
    _MBX_HIGHLIGHT_PLAIN=${READLINE_LINE-}
    _MBX_HIGHLIGHT_POINT=${READLINE_POINT:-0}
}

_mbx_highlight_self_insert() {
    local ch=${1-}
    local plain point

    [[ -n $ch ]] || ch=${READLINE_KEYSEQ-}
    [[ -n $ch ]] || return 0
    _mbx_text_has_c0_or_del "$ch" && return 0
    _mbx_highlight_sync_plain
    plain=${_MBX_HIGHLIGHT_PLAIN-}
    point=${_MBX_HIGHLIGHT_POINT:-0}
    READLINE_LINE=${plain:0:point}${ch}${plain:point}
    READLINE_POINT=$((point + ${#ch}))
    _MBX_HIGHLIGHT_PLAIN=$READLINE_LINE
    _MBX_HIGHLIGHT_POINT=$READLINE_POINT
    if ! _mbx_highlight_refresh; then
        _mbx_highlight_fallback_plain
    fi
}

_mbx_highlight_backspace() {
    local plain point

    _mbx_highlight_sync_plain
    plain=${_MBX_HIGHLIGHT_PLAIN-}
    point=${_MBX_HIGHLIGHT_POINT:-0}
    if ((point > 0)); then
        READLINE_LINE=${plain:0:point-1}${plain:point}
        READLINE_POINT=$((point - 1))
        _MBX_HIGHLIGHT_PLAIN=$READLINE_LINE
        _MBX_HIGHLIGHT_POINT=$READLINE_POINT
        if ! _mbx_highlight_refresh; then
            _mbx_highlight_fallback_plain
        fi
    fi
}

_mbx_highlight_forward() {
    local point len

    _mbx_highlight_sync_plain
    point=${_MBX_HIGHLIGHT_POINT:-0}
    len=${#_MBX_HIGHLIGHT_PLAIN}
    if ((point < len)); then
        _MBX_HIGHLIGHT_POINT=$((point + 1))
        READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
        READLINE_POINT=${_MBX_HIGHLIGHT_POINT}
        if ! _mbx_highlight_refresh; then
            _mbx_highlight_fallback_plain
        fi
    fi
}

_mbx_highlight_backward() {
    local point

    _mbx_highlight_sync_plain
    point=${_MBX_HIGHLIGHT_POINT:-0}
    if ((point > 0)); then
        _MBX_HIGHLIGHT_POINT=$((point - 1))
        READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
        READLINE_POINT=${_MBX_HIGHLIGHT_POINT}
        if ! _mbx_highlight_refresh; then
            _mbx_highlight_fallback_plain
        fi
    fi
}

_mbx_highlight_beginning() {
    _mbx_highlight_sync_plain
    _MBX_HIGHLIGHT_POINT=0
    READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
    READLINE_POINT=0
    if ! _mbx_highlight_refresh; then
        _mbx_highlight_fallback_plain
    fi
}

_mbx_highlight_quoted_keyseq() {
    local ch=$1
    local LC_ALL=C
    case $ch in
        \\) REPLY='"\\"' ;;
        \") REPLY='"\""' ;;
        \`) REPLY='"`"' ;;
        *) REPLY="\"${ch}\"" ;;
    esac
}

_mbx_highlight_keyseq_has_x() {
    local keyseq=$1
    local keymap=$2
    local quoted
    _mbx_highlight_quoted_keyseq "$keyseq"
    quoted=$REPLY
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "${quoted}:"
}

_mbx_highlight_stock_fn() {
    local keyseq=$1
    local keymap=$2
    local quoted line=
    _mbx_highlight_quoted_keyseq "$keyseq"
    quoted=$REPLY
    line=$(bind -m "$keymap" -p 2>/dev/null | grep -F "${quoted}:" | head -n 1) || \
        line=
    REPLY=${line##*: }
    REPLY=${REPLY# }
}

_mbx_highlight_can_wrap() {
    local keyseq=$1
    local keymap=$2
    local allowed=$3
    local fn
    if _mbx_highlight_keyseq_has_x "$keyseq" "$keymap"; then
        [[ ${MBX_HIGHLIGHT_OVERRIDE:-0} == 1 ]] || return 1
        return 0
    fi
    _mbx_highlight_stock_fn "$keyseq" "$keymap"
    fn=$REPLY
    [[ -z $fn || $fn == "$allowed" ]]
}

_mbx_highlight_bind_x() {
    local keymap=$1
    local keyseq=$2
    local command=$3
    local allowed=$4
    local spec
    _mbx_highlight_can_wrap "$keyseq" "$keymap" "$allowed" || return 1
    _mbx_highlight_quoted_keyseq "$keyseq"
    spec="${REPLY}: $command"
    bind -m "$keymap" -x "$spec"
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
    local wrapped=0

    _mbx_highlight_bind_self_chars "$keymap" "$chars" && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-h' _mbx_highlight_backspace backward-delete-char \
        && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-?' _mbx_highlight_backspace backward-delete-char \
        && wrapped=1
    _mbx_highlight_bind_x "$keymap" '\C-f' _mbx_highlight_forward forward-char || true
    _mbx_highlight_bind_x "$keymap" '\e[C' _mbx_highlight_forward forward-char || true
    _mbx_highlight_bind_x "$keymap" '\C-b' _mbx_highlight_backward backward-char || true
    _mbx_highlight_bind_x "$keymap" '\e[D' _mbx_highlight_backward backward-char || true
    _mbx_highlight_bind_x "$keymap" '\eOD' _mbx_highlight_backward backward-char || true
    _mbx_highlight_bind_x "$keymap" '\C-a' _mbx_highlight_beginning beginning-of-line || true
    _mbx_highlight_bind_x "$keymap" '\eOH' _mbx_highlight_beginning beginning-of-line || true
    _mbx_highlight_bind_x "$keymap" '\e[H' _mbx_highlight_beginning beginning-of-line || true
    ((wrapped == 1))
}

_mbx_highlight_install() {
    [[ ${_MBX_HIGHLIGHT_INSTALLED:-0} != 1 ]] || return 0
    _MBX_HIGHLIGHT_BOUND=0
    _MBX_HIGHLIGHT_VI_BOUND=0
    _MBX_HIGHLIGHT_PAINTED=0
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
    if _mbx_highlight_install_keymap emacs; then
        _MBX_HIGHLIGHT_BOUND=1
    fi
    if _mbx_highlight_install_keymap vi-insert; then
        _MBX_HIGHLIGHT_VI_BOUND=1
    fi
    _MBX_HIGHLIGHT_INSTALLED=1
}
