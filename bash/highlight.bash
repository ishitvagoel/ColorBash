# shellcheck shell=bash
# Opt-in syntax highlighting (ADR 0013). Plain bytes live in _MBX_HIGHLIGHT_PLAIN;
# READLINE_LINE holds styled text with Readline non-printing markers. Enter
# arms a Readline-only restore macro (M-041); do not combine with MBX_GHOST=1.

_MBX_HIGHLIGHT_ACCEPT_DEFAULT_KEYSEQ='\C-x\C-m'
_MBX_HIGHLIGHT_PLAIN=
_MBX_HIGHLIGHT_POINT=0
_MBX_HIGHLIGHT_ACTIVE=0
_MBX_HIGHLIGHT_BOUND=0
_MBX_HIGHLIGHT_VI_BOUND=0
_MBX_HIGHLIGHT_ENTER_ARMED=0
_MBX_HIGHLIGHT_WRAP_CTRL_J=0
_MBX_HIGHLIGHT_VI_WRAP_CTRL_J=0
_MBX_HIGHLIGHT_ACCEPT_KEYSEQ=$_MBX_HIGHLIGHT_ACCEPT_DEFAULT_KEYSEQ
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

_mbx_highlight_styled_to_plain_point() {
    local line=${READLINE_LINE-}
    local styled_point=${READLINE_POINT:-0}
    local index=0 plain_index=0
    local skipping=0 byte code
    local LC_ALL=C

    while ((index < styled_point && index < ${#line})); do
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
            ((plain_index++))
        fi
        index=$((index + 1))
    done
    REPLY=$plain_index
}

_mbx_highlight_escape_macro_plain() {
    local plain=$1
    local i char escaped=
    for ((i = 0; i < ${#plain}; i++)); do
        char=${plain:i:1}
        case $char in
            \\) escaped+='\\' ;;
            \") escaped+='\\"' ;;
            *) escaped+=$char ;;
        esac
    done
    REPLY=$escaped
}

_mbx_highlight_enter_restore_macro() {
    local plain=$1
    local accept_key=$2
    local escaped
    _mbx_highlight_escape_macro_plain "$plain"
    escaped=$REPLY
    REPLY=$'\C-u'"$escaped$accept_key"
}

_mbx_highlight_disarm_enter_keymap() {
    local keymap=$1
    local wrap_j=$2
    bind -m "$keymap" '"\C-m": accept-line' || return 1
    if [[ $wrap_j == 1 ]]; then
        bind -m "$keymap" '"\C-j": accept-line' || return 1
    fi
}

_mbx_highlight_arm_enter_keymap() {
    local keymap=$1
    local wrap_j=$2
    local macro=$3
    bind -m "$keymap" "\"\\C-m\": \"$macro\"" || return 1
    if [[ $wrap_j == 1 ]]; then
        if ! bind -m "$keymap" "\"\\C-j\": \"$macro\""; then
            bind -m "$keymap" '"\C-m": accept-line' || true
            return 1
        fi
    fi
}

_mbx_highlight_disarm_enter() {
    [[ ${_MBX_HIGHLIGHT_ENTER_ARMED:-0} == 1 ]] || return 0
    local status=0
    _mbx_highlight_disarm_enter_keymap emacs "${_MBX_HIGHLIGHT_WRAP_CTRL_J:-0}" || status=1
    if [[ ${_MBX_HIGHLIGHT_VI_BOUND:-0} == 1 ]]; then
        _mbx_highlight_disarm_enter_keymap vi-insert "${_MBX_HIGHLIGHT_VI_WRAP_CTRL_J:-0}" || \
            status=1
    fi
    # Always clear the flag so a later arm can retry. Returning before this
    # left emacs disarmed while Enter stayed the restore macro (M-044).
    _MBX_HIGHLIGHT_ENTER_ARMED=0
    return "$status"
}

_mbx_highlight_arm_enter() {
    local macro plain=${_MBX_HIGHLIGHT_PLAIN-}
    local accept_key=${_MBX_HIGHLIGHT_ACCEPT_KEYSEQ}
    [[ ${_MBX_HIGHLIGHT_BOUND:-0} == 1 ]] || return 0
    if [[ ${_MBX_HIGHLIGHT_ENTER_ARMED:-0} == 1 ]]; then
        _mbx_highlight_disarm_enter || return 1
    fi
    _mbx_highlight_enter_restore_macro "$plain" "$accept_key" || return 1
    macro=$REPLY
    [[ -n $macro ]] || return 1
    _mbx_highlight_arm_enter_keymap emacs "${_MBX_HIGHLIGHT_WRAP_CTRL_J:-0}" "$macro" || \
        return 1
    if [[ ${_MBX_HIGHLIGHT_VI_BOUND:-0} == 1 ]]; then
        if ! _mbx_highlight_arm_enter_keymap vi-insert "${_MBX_HIGHLIGHT_VI_WRAP_CTRL_J:-0}" \
            "$macro"; then
            _mbx_highlight_disarm_enter_keymap emacs "${_MBX_HIGHLIGHT_WRAP_CTRL_J:-0}" || true
            return 1
        fi
    fi
    _MBX_HIGHLIGHT_ENTER_ARMED=1
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

_mbx_highlight_fallback_plain() {
    READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
    READLINE_POINT=${_MBX_HIGHLIGHT_POINT:-0}
    _MBX_HIGHLIGHT_ACTIVE=0
    _mbx_highlight_disarm_enter || true
}

# Spawn fallback: one helper process per call. Used only when no coprocess is
# attached (MBX_IPC_MODE=off/per-call, or the coprocess died this cycle).
_mbx_highlight_refresh_cli() {
    (($# == 3)) || return 2

    local plain=$1 point=$2 deadline=$3
    local output_fd child_pid color styled_line styled_point

    # Always 0: see the comment in _mbx_highlight_refresh_wire (M-064).
    color=0
    exec {output_fd}< <(exec "$MBX_BIN" highlight "$plain" --point "$point" \
        --color "$color" 2>/dev/null)
    child_pid=$!
    if ! _mbx_highlight_read_two_lines "$output_fd" "$deadline"; then
        exec {output_fd}<&-
        _mbx_wait_or_kill_child "$child_pid" "$deadline" || true
        return 1
    fi
    # Copy the payload out of REPLY/_MBX_HIGHLIGHT_STYLED_POINT before
    # _mbx_wait_child_until, which overwrites REPLY with an exit status
    # (M-049/M-055).
    styled_line=$REPLY
    styled_point=${_MBX_HIGHLIGHT_STYLED_POINT:-0}
    exec {output_fd}<&-
    if ! _mbx_wait_or_kill_child "$child_pid" "$deadline"; then
        return 1
    fi
    # A helper that wrote two lines then exited nonzero (or was killed after
    # a partial write that still parsed) is not a successful highlight
    # (M-067). validate_styled cannot see the exit status.
    ((REPLY == 0)) || return 1
    REPLY=$styled_line
    _MBX_HIGHLIGHT_STYLED_POINT=$styled_point
    return 0
}

# Coprocess path: one HIGHLIGHT/STYLED round trip over the already-warm
# transport, matching ghost's QUERY/RESULT shape (ADR 0011 generation and
# stale-reply skip; ADR 0014 extends the same discipline to highlighting). A
# delayed STYLED reply for an older generation is skipped rather than treated
# as a hard failure, so a backed-up coprocess cannot desync the line buffer.
_mbx_highlight_refresh_wire() {
    (($# == 4)) || return 2

    local plain=$1 point=$2 generation=$3 deadline=$4
    local request_id response result_gen color
    local -a fields=()

    # Always 0: Bash's own Readline redisplay renders \001/\002 using its
    # standard unprintable-control-character convention (caret notation, e.g.
    # `^A^[[35m^B`) when they appear inside READLINE_LINE, unlike their
    # documented zero-width behavior inside PS1. Passing a real color
    # decision here would replace plain typed text with visibly garbled
    # output on every keystroke. See M-064; fixing this needs a follow-up ADR
    # on a rendering technique Readline actually hides within the edit
    # buffer, not just a wire-protocol change.
    color=0
    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID
    _mbx_protocol_encode_highlight "$request_id" "$generation" "$color" "$point" "$plain" || \
        return 1
    if ! _mbx_engine_write "$REPLY" "$deadline"; then
        # A failed write can leave the coprocess desynced; stop so the prompt
        # path starts a clean helper next cycle (matches the ghost wire path).
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
        # A history RECORD's ACK can still be queued on the shared fd when a
        # keystroke lands mid-cycle: MBX_HIGHLIGHT=1 and MBX_HISTORY=1 are not
        # mutually exclusive (only ghost and highlight are), and both features
        # read the one coprocess. Skip it the way ghost's identical loop does
        # rather than tearing down a healthy helper.
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
    local styled_line styled_point status=1

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
    styled_point=${_MBX_HIGHLIGHT_STYLED_POINT:-0}
    _mbx_highlight_validate_styled "$styled_line" || return 1
    READLINE_LINE=$styled_line
    READLINE_POINT=$styled_point
    _MBX_HIGHLIGHT_ACTIVE=1
    if [[ $styled_line == "$plain" ]]; then
        _mbx_highlight_disarm_enter || true
    elif ! _mbx_highlight_arm_enter; then
        _mbx_highlight_fallback_plain
        return 1
    fi
    return 0
}

_mbx_highlight_capture_plain() {
    if [[ ${_MBX_HIGHLIGHT_ACTIVE:-0} == 1 ]]; then
        _mbx_highlight_strip_line "${READLINE_LINE-}"
        if [[ $REPLY == "${_MBX_HIGHLIGHT_PLAIN-}" && ${READLINE_LINE-} == "$REPLY" ]]; then
            _MBX_HIGHLIGHT_POINT=${READLINE_POINT:-0}
        else
            _mbx_highlight_styled_to_plain_point
            _MBX_HIGHLIGHT_POINT=$REPLY
        fi
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
        _mbx_highlight_fallback_plain
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
            _mbx_highlight_fallback_plain
        fi
    fi
}

_mbx_highlight_dismiss_style() {
    if [[ ${_MBX_HIGHLIGHT_ACTIVE:-0} != 1 ]]; then
        return 0
    fi
    _mbx_highlight_styled_to_plain_point
    _MBX_HIGHLIGHT_POINT=$REPLY
    READLINE_LINE=${_MBX_HIGHLIGHT_PLAIN-}
    READLINE_POINT=${_MBX_HIGHLIGHT_POINT:-0}
    _MBX_HIGHLIGHT_ACTIVE=0
    _mbx_highlight_disarm_enter || true
}

_mbx_highlight_forward() {
    _mbx_highlight_dismiss_style
    local point=${_MBX_HIGHLIGHT_POINT:-0}
    local len=${#_MBX_HIGHLIGHT_PLAIN}
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
    _mbx_highlight_dismiss_style
    local point=${_MBX_HIGHLIGHT_POINT:-0}
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
    _mbx_highlight_dismiss_style
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

_mbx_highlight_bind_fn() {
    local keymap=$1
    local keyseq=$2
    local fn=$3
    local allowed=$4
    local spec
    _mbx_highlight_can_wrap "$keyseq" "$keymap" "$allowed" || return 1
    _mbx_highlight_quoted_keyseq "$keyseq"
    spec="${REPLY}: $fn"
    bind -m "$keymap" "$spec"
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
    local wrapped=0 wrap_j=0

    _mbx_highlight_can_wrap "$accept_key" "$keymap" accept-line || return 1
    _mbx_highlight_stock_fn '\C-m' "$keymap"
    if [[ $REPLY != accept-line ]] || _mbx_highlight_keyseq_has_x '\C-m' "$keymap"; then
        return 1
    fi
    _mbx_highlight_stock_fn '\C-j' "$keymap"
    if [[ $REPLY == accept-line ]] && ! _mbx_highlight_keyseq_has_x '\C-j' "$keymap"; then
        wrap_j=1
    fi
    ((wrap_j == 1)) || return 1
    _mbx_highlight_bind_fn "$keymap" "$accept_key" accept-line accept-line || return 1
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
    if [[ $keymap == emacs ]]; then
        _MBX_HIGHLIGHT_WRAP_CTRL_J=$wrap_j
    else
        _MBX_HIGHLIGHT_VI_WRAP_CTRL_J=$wrap_j
    fi
    ((wrapped == 1))
}

_mbx_highlight_install() {
    [[ ${_MBX_HIGHLIGHT_INSTALLED:-0} != 1 ]] || return 0
    _MBX_HIGHLIGHT_BOUND=0
    _MBX_HIGHLIGHT_VI_BOUND=0
    _MBX_HIGHLIGHT_ENTER_ARMED=0
    _MBX_HIGHLIGHT_WRAP_CTRL_J=0
    _MBX_HIGHLIGHT_VI_WRAP_CTRL_J=0
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
