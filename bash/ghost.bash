# shellcheck shell=bash
# Opt-in inline ghost suffix (ADR 0010). Suggestion lives after READLINE_POINT.
# `\C-xg` is glob-list-expansions (M-040). Enter is not bind -x: a -x step in a
# keyseq macro drops the remaining keys (M-041), so an active suffix arms a
# Readline-only delete-char + accept-line macro on `\C-m` and `\C-j` instead.
# Cycle next/prev default to unbound `\C-x\C-n` / `\C-x\C-p` (not `\en`/`\ep`).
# Remaining ASCII printables use Readline quoted keyseqs so `"` / `\` bind.
# vi-insert is wrapped after emacs; ESC is vi-movement-mode so `\ef` is not.
# Left / `\C-b` strip an unaccepted suffix then backward-char (Home stays later).
# Home / Up / Down / backward-word use the same strip-first dismiss policy.
_MBX_GHOST_ACCEPT_DEFAULT_KEYSEQ='\C-x\C-m'
_MBX_GHOST_DELETE_DEFAULT_KEYSEQ='\C-x\C-d'
_MBX_GHOST_SUFFIX_MAX=256
_MBX_GHOST_NEXT_DEFAULT_KEYSEQ='\C-x\C-n'
_MBX_GHOST_PREV_DEFAULT_KEYSEQ='\C-x\C-p'
_MBX_GHOST_HAS=0
_MBX_GHOST_POINT=0
_MBX_GHOST_BOUND=0
_MBX_GHOST_VI_BOUND=0
_MBX_GHOST_CYCLE_BOUND=0
_MBX_GHOST_ENTER_ARMED=0
_MBX_GHOST_WRAP_CTRL_J=0
_MBX_GHOST_VI_WRAP_CTRL_J=0
_MBX_GHOST_INDEX=0
_MBX_GHOST_TYPED_LEN=0
_MBX_GHOST_HIST_OFFSET=0
_MBX_GHOST_HIST_CURRENT=
_MBX_GHOST_CANDIDATES=()
_MBX_GHOST_GENERATION=0
_MBX_GHOST_DELETE_KEYSEQ=$_MBX_GHOST_DELETE_DEFAULT_KEYSEQ
_MBX_GHOST_ACCEPT_KEYSEQ=$_MBX_GHOST_ACCEPT_DEFAULT_KEYSEQ

_mbx_ghost_enter_delete_macro() {
    local n=${1:-0}
    local i macro=''
    local max=${_MBX_GHOST_SUFFIX_MAX:-256}
    if ((n < 1)); then
        REPLY=
        return 0
    fi
    if ((n > max)); then
        n=$max
    fi
    for ((i = 0; i < n; i++)); do
        macro+="${_MBX_GHOST_DELETE_KEYSEQ}"
    done
    macro+="${_MBX_GHOST_ACCEPT_KEYSEQ}"
    REPLY=$macro
}

_mbx_ghost_disarm_enter_keymap() {
    local keymap=$1
    local wrap_j=$2
    bind -m "$keymap" '"\C-m": accept-line' || return 1
    if [[ $wrap_j == 1 ]]; then
        bind -m "$keymap" '"\C-j": accept-line' || return 1
    fi
}

_mbx_ghost_arm_enter_keymap() {
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

_mbx_ghost_disarm_enter() {
    [[ ${_MBX_GHOST_ENTER_ARMED:-0} == 1 ]] || return 0
    local status=0
    _mbx_ghost_disarm_enter_keymap emacs "${_MBX_GHOST_WRAP_CTRL_J:-0}" || status=1
    if [[ ${_MBX_GHOST_VI_BOUND:-0} == 1 ]]; then
        _mbx_ghost_disarm_enter_keymap vi-insert "${_MBX_GHOST_VI_WRAP_CTRL_J:-0}" || \
            status=1
    fi
    # Always clear the flag so a later arm can retry. Returning before this
    # left emacs disarmed while Enter stayed the discard macro (M-044).
    _MBX_GHOST_ENTER_ARMED=0
    return "$status"
}

_mbx_ghost_arm_enter() {
    local suffix_len=${1:-0}
    local macro
    [[ ${_MBX_GHOST_BOUND:-0} == 1 ]] || return 0
    ((suffix_len > 0)) || return 0
    if [[ ${_MBX_GHOST_ENTER_ARMED:-0} == 1 ]]; then
        _mbx_ghost_disarm_enter || return 1
    fi
    _mbx_ghost_enter_delete_macro "$suffix_len" || return 1
    macro=$REPLY
    [[ -n $macro ]] || return 1
    _mbx_ghost_arm_enter_keymap emacs "${_MBX_GHOST_WRAP_CTRL_J:-0}" "$macro" || \
        return 1
    if [[ ${_MBX_GHOST_VI_BOUND:-0} == 1 ]]; then
        if ! _mbx_ghost_arm_enter_keymap vi-insert "${_MBX_GHOST_VI_WRAP_CTRL_J:-0}" \
            "$macro"; then
            _mbx_ghost_disarm_enter_keymap emacs "${_MBX_GHOST_WRAP_CTRL_J:-0}" || \
                true
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

_mbx_ghost_reset_hist_offset() {
    _MBX_GHOST_HIST_OFFSET=0
    _MBX_GHOST_HIST_CURRENT=
}

_mbx_ghost_clear() {
    _mbx_ghost_reset_state
    _mbx_ghost_reset_hist_offset
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

# Kept as a thin, stable name for ghost call sites; the bounded reader itself
# is shared with highlight's coprocess path (M-062) and lives in engine.bash.
_mbx_ghost_read_line() {
    _mbx_engine_read_line "$1" "$2"
}

_mbx_ghost_usable_match() {
    local typed=$1
    local match=$2
    local suffix

    [[ -n $typed && -n $match ]] || return 1
    [[ $match == "$typed"* ]] || return 1
    suffix=${match#"$typed"}
    [[ -n $suffix ]] || return 1
    ((${#suffix} <= 256)) || return 1
    _mbx_text_has_c0_or_del "$suffix" && return 1
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

_mbx_ghost_restore_jobs() {
    _mbx_jobs_restore
}

# Apply a QUERY RESULT only when its generation is still current (ADR 0011).
_mbx_ghost_accept_commands() {
    (($# == 4)) || return 2

    local result_gen=$1
    local query=$2
    local limit=$3
    local -n _mbx_ghost_cmds=$4
    local match

    _MBX_GHOST_CANDIDATES=()
    _MBX_GHOST_INDEX=0
    ((result_gen == _MBX_GHOST_GENERATION)) || return 1
    if ((${#_mbx_ghost_cmds[@]} > 0)); then
        for match in "${_mbx_ghost_cmds[@]}"; do
            ((${#_MBX_GHOST_CANDIDATES[@]} < limit)) || break
            _mbx_ghost_usable_match "$query" "$match" || continue
            match=$REPLY
            _mbx_ghost_candidate_seen "$match" && continue
            _MBX_GHOST_CANDIDATES+=("$match")
        done
    fi
    ((${#_MBX_GHOST_CANDIDATES[@]} > 0)) || return 1
    REPLY=${_MBX_GHOST_CANDIDATES[0]}
}

_mbx_ghost_query_wire() {
    (($# == 4)) || return 2

    local query=$1
    local limit=$2
    local generation=$3
    local deadline=$4
    local request_id response result_gen
    local -a cmds=() fields=()

    ((_MBX_REQUEST_ID += 1))
    request_id=$_MBX_REQUEST_ID
    _mbx_protocol_encode_history_query \
        "$request_id" "$generation" prefix "$query" "$limit" || return 1
    if ! _mbx_engine_write "$REPLY" "$deadline"; then
        # A failed write can leave the coprocess desynced; stop so the prompt
        # path can start a clean helper.
        _mbx_engine_stop
        return 1
    fi
    # Do not stop on a timed-out read: a delayed RESULT for this or an older
    # generation may still arrive, and the next QUERY must skip it (ADR 0011).
    # Hard decode failures still stop. RECORD/prompt already stop on a stale
    # frame, so an idle delayed RESULT cannot poison a later PS1 cycle.
    # `_mbx_read_bounded_response` can slurp the next queued RESULT in the same
    # acquisition and then reject the extra LF (M-045). RESULT skip needs a
    # line reader that leaves later frames in the pipe.
    while _mbx_deadline_remaining "$deadline" >/dev/null; do
        if ! _mbx_ghost_read_line "$_MBX_ENGINE_OUT_FD" "$deadline"; then
            return 1
        fi
        response=$REPLY
        if _mbx_protocol_parse_history_result "$response" cmds; then
            result_gen=$REPLY
            if ((result_gen == generation)); then
                _mbx_ghost_accept_commands "$result_gen" "$query" "$limit" cmds
                return $?
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

_mbx_ghost_query_cli() {
    (($# == 3)) || return 2

    local query=$1
    local limit=$2
    local deadline=$3
    local output_fd child_pid match=

    exec {output_fd}< <(exec "$MBX_BIN" history search prefix "$query" --limit "$limit" \
        2>/dev/null) || return 1
    child_pid=$!
    _MBX_GHOST_CANDIDATES=()
    _MBX_GHOST_INDEX=0
    while ((${#_MBX_GHOST_CANDIDATES[@]} < limit)); do
        _mbx_ghost_read_line "$output_fd" "$deadline" || break
        match=$REPLY
        _mbx_ghost_usable_match "$query" "$match" || continue
        match=$REPLY
        _mbx_ghost_candidate_seen "$match" && continue
        _MBX_GHOST_CANDIDATES+=("$match")
    done
    exec {output_fd}<&-
    if ! _mbx_wait_or_kill_child "$child_pid" "$deadline"; then
        _MBX_GHOST_CANDIDATES=()
        return 1
    fi
    # A timed-out/killed child is already rejected above. A nonzero exit with
    # no candidates is also a miss; leftover rows from a successful parse
    # still count, matching search's "any clean match is enough" policy.
    ((${#_MBX_GHOST_CANDIDATES[@]} > 0)) || return 1
    REPLY=${_MBX_GHOST_CANDIDATES[0]}
}

_mbx_ghost_query() {
    local query=$1
    local deadline limit status=1

    _MBX_GHOST_CANDIDATES=()
    _MBX_GHOST_INDEX=0
    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ -n $query ]] || return 1
    _mbx_ghost_limit
    limit=$REPLY
    _mbx_deadline_after "${MBX_GHOST_TIMEOUT:-${MBX_HISTORY_TIMEOUT:-0.10}}" || \
        return 1
    deadline=$REPLY
    ((_MBX_GHOST_GENERATION += 1))
    _mbx_jobs_suspend
    if [[ ${_MBX_ENGINE_READY:-0} == 1 ]] && \
        declare -F _mbx_engine_write >/dev/null 2>&1; then
        _mbx_ghost_query_wire "$query" "$limit" "$_MBX_GHOST_GENERATION" \
            "$deadline" && status=0
    else
        _mbx_ghost_query_cli "$query" "$limit" "$deadline" && status=0
    fi
    _mbx_ghost_restore_jobs
    return "$status"
}

_mbx_ghost_show() {
    local match=$1
    local typed=$2
    local point=${_MBX_GHOST_TYPED_LEN:-0}

    READLINE_LINE=$match
    READLINE_POINT=$point
    _MBX_GHOST_HAS=1
    _MBX_GHOST_POINT=$point
    if [[ ${_MBX_GHOST_BOUND:-0} == 1 ]]; then
        local suffix_len=$((${#match} - point))
        _mbx_ghost_disarm_enter || true
        if ! _mbx_ghost_arm_enter "$suffix_len" || \
            [[ ${_MBX_GHOST_ENTER_ARMED:-0} != 1 ]]; then
            READLINE_LINE=$typed
            READLINE_POINT=$point
            _mbx_ghost_reset_state
            return 1
        fi
    fi
}

_mbx_ghost_refresh() {
    local typed=${READLINE_LINE-}
    local point=${READLINE_POINT:-0}

    _mbx_ghost_reset_state
    _mbx_ghost_reset_hist_offset
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
    _mbx_ghost_reset_hist_offset
    _mbx_ghost_strip
    _mbx_ghost_insert_char "$ch"
    _mbx_ghost_refresh
}

_mbx_ghost_backspace() {
    local point=${READLINE_POINT:-0}
    local line=${READLINE_LINE-}
    _mbx_ghost_reset_hist_offset
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

_mbx_ghost_backward() {
    local point=${READLINE_POINT:-0}
    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        _mbx_ghost_strip
        point=${READLINE_POINT:-0}
    fi
    if ((point > 0)); then
        READLINE_POINT=$((point - 1))
    fi
}

_mbx_ghost_beginning() {
    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        _mbx_ghost_strip
    fi
    READLINE_POINT=0
}

_mbx_ghost_history_entry() {
    local offset=$1
    local entry

    ((offset >= 1)) || return 1
    entry=$(HISTTIMEFORMAT= history "$offset" 2>/dev/null | head -n 1) || return 1
    # Same list-number parse as `_mbx_history_parse_latest` (M-026 / M-047).
    # `${entry#*[0-9]}` only strips the first digit, so `12  echo` became `2  echo`.
    if [[ $entry =~ ^[[:space:]]*[0-9]+[[:space:]][[:space:]](.*)$ ]]; then
        REPLY=${BASH_REMATCH[1]}
        [[ -n $REPLY ]] || return 1
        return 0
    fi
    return 1
}

_mbx_ghost_previous_history() {
    local offset entry

    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        _mbx_ghost_strip
    fi
    if ((${_MBX_GHOST_HIST_OFFSET:-0} == 0)); then
        _MBX_GHOST_HIST_CURRENT=${READLINE_LINE-}
    fi
    offset=$((${_MBX_GHOST_HIST_OFFSET:-0} + 1))
    while true; do
        _mbx_ghost_history_entry "$offset" || return 0
        entry=$REPLY
        # History-motion is a READLINE_LINE sink. QUERY suffixes are already
        # gated; Up/Down were not, so a TAB/ESC in a history row reached
        # redisplay (M-050).
        if _mbx_text_has_c0_or_del "$entry"; then
            offset=$((offset + 1))
            continue
        fi
        break
    done
    READLINE_LINE=$entry
    READLINE_POINT=${#READLINE_LINE}
    _MBX_GHOST_HIST_OFFSET=$offset
    _mbx_ghost_reset_state
    _mbx_ghost_disarm_enter || true
}

_mbx_ghost_next_history() {
    local offset entry

    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        _mbx_ghost_strip
    fi
    offset=${_MBX_GHOST_HIST_OFFSET:-0}
    if ((offset <= 0)); then
        return 0
    fi
    while ((offset > 0)); do
        offset=$((offset - 1))
        _MBX_GHOST_HIST_OFFSET=$offset
        if ((offset == 0)); then
            READLINE_LINE=${_MBX_GHOST_HIST_CURRENT-}
            READLINE_POINT=${#READLINE_LINE}
            _mbx_ghost_reset_state
            _mbx_ghost_disarm_enter || true
            return 0
        fi
        _mbx_ghost_history_entry "$offset" || return 0
        entry=$REPLY
        if _mbx_text_has_c0_or_del "$entry"; then
            continue
        fi
        READLINE_LINE=$entry
        READLINE_POINT=${#READLINE_LINE}
        _mbx_ghost_reset_state
        _mbx_ghost_disarm_enter || true
        return 0
    done
}

_mbx_ghost_backward_word() {
    local line=${READLINE_LINE-}
    local point=${READLINE_POINT:-0}
    local c
    local LC_ALL=C

    if [[ ${_MBX_GHOST_HAS:-0} == 1 ]]; then
        _mbx_ghost_strip
        point=${READLINE_POINT:-0}
        line=${READLINE_LINE-}
    fi
    while ((point > 0)); do
        c=${line:point-1:1}
        [[ $c == [[:alnum:]] ]] && break
        point=$((point - 1))
    done
    while ((point > 0)); do
        c=${line:point-1:1}
        [[ $c == [[:alnum:]] ]] || break
        point=$((point - 1))
    done
    READLINE_POINT=$point
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
        \`) REPLY='"`"' ;;
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

_mbx_ghost_bind_x_stock() {
    local keymap=$1
    local keyseq=$2
    local command=$3
    local allowed=$4
    local fn spec
    if _mbx_ghost_keyseq_has_x "$keyseq" "$keymap"; then
        [[ ${MBX_GHOST_OVERRIDE:-0} == 1 ]] || return 1
    else
        _mbx_ghost_stock_fn "$keyseq" "$keymap"
        fn=$REPLY
        [[ $fn == "$allowed" ]] || return 1
    fi
    _mbx_ghost_quoted_keyseq "$keyseq"
    spec="${REPLY}: $command"
    bind -m "$keymap" -x "$spec"
}

_mbx_ghost_install_vi() {
    local delete_key=$1
    local accept_key=$2
    local next_key=$3
    local prev_key=$4
    local chars=$5
    _MBX_GHOST_VI_WRAP_CTRL_J=0
    _mbx_ghost_can_wrap "$delete_key" vi-insert delete-char || return 1
    _mbx_ghost_can_wrap "$accept_key" vi-insert accept-line || return 1
    _mbx_ghost_stock_fn '\C-m' vi-insert
    if [[ $REPLY != accept-line ]] || _mbx_ghost_keyseq_has_x '\C-m' vi-insert; then
        return 1
    fi
    _mbx_ghost_stock_fn '\C-j' vi-insert
    if [[ $REPLY == accept-line ]] && ! _mbx_ghost_keyseq_has_x '\C-j' vi-insert; then
        _MBX_GHOST_VI_WRAP_CTRL_J=1
    fi
    [[ ${_MBX_GHOST_VI_WRAP_CTRL_J:-0} == 1 ]] || return 1
    _mbx_ghost_bind_fn vi-insert "$delete_key" delete-char delete-char || return 1
    _mbx_ghost_bind_fn vi-insert "$accept_key" accept-line accept-line || return 1
    _mbx_ghost_bind_self_chars vi-insert "$chars" || return 1
    _mbx_ghost_bind_x vi-insert '\C-h' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x vi-insert '\C-?' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x vi-insert '\e[C' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x vi-insert '\eOC' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x vi-insert '\e[D' _mbx_ghost_backward backward-char || true
    _mbx_ghost_bind_x vi-insert '\eOD' _mbx_ghost_backward backward-char || true
    _mbx_ghost_bind_x vi-insert '\eOH' _mbx_ghost_beginning beginning-of-line || true
    _mbx_ghost_bind_x vi-insert '\e[H' _mbx_ghost_beginning beginning-of-line || true
    _mbx_ghost_bind_x vi-insert '\eOA' _mbx_ghost_previous_history previous-history || \
        true
    _mbx_ghost_bind_x vi-insert '\e[A' _mbx_ghost_previous_history previous-history || \
        true
    _mbx_ghost_bind_x vi-insert '\eOB' _mbx_ghost_next_history next-history || true
    _mbx_ghost_bind_x vi-insert '\e[B' _mbx_ghost_next_history next-history || true
    _mbx_ghost_bind_x_stock vi-insert '\e[1;5D' _mbx_ghost_backward_word backward-word || \
        true
    _mbx_ghost_bind_x_stock vi-insert '\e[1;5C' _mbx_ghost_forward_word forward-word || \
        true
    if [[ -n $next_key && $next_key != "$delete_key" && $next_key != "$accept_key" ]]; then
        _mbx_ghost_bind_x vi-insert "$next_key" _mbx_ghost_cycle_next '' || true
    fi
    if [[ -n $prev_key && $prev_key != "$delete_key" && $prev_key != "$accept_key" && $prev_key != "$next_key" ]]; then
        _mbx_ghost_bind_x vi-insert "$prev_key" _mbx_ghost_cycle_prev '' || true
    fi
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
    _MBX_GHOST_VI_BOUND=0
    _MBX_GHOST_CYCLE_BOUND=0
    _MBX_GHOST_ENTER_ARMED=0
    _MBX_GHOST_WRAP_CTRL_J=0
    _MBX_GHOST_VI_WRAP_CTRL_J=0
    if [[ $- != *i* || ! -t 0 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    if [[ ${MBX_GHOST:-} != 1 || ${MBX_HISTORY:-} != 1 ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    local delete_key=${MBX_GHOST_DELETE_KEYSEQ:-$_MBX_GHOST_DELETE_DEFAULT_KEYSEQ}
    local accept_key=${MBX_GHOST_ACCEPT_KEYSEQ:-$_MBX_GHOST_ACCEPT_DEFAULT_KEYSEQ}
    local next_key=${MBX_GHOST_NEXT_KEYSEQ:-$_MBX_GHOST_NEXT_DEFAULT_KEYSEQ}
    local prev_key=${MBX_GHOST_PREV_KEYSEQ:-$_MBX_GHOST_PREV_DEFAULT_KEYSEQ}
    local chars=$'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-.:/!#$%&()*+,;<=>?@[]^{|}~\'"`\\'
    [[ -n $delete_key && -n $accept_key ]] || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    if [[ $delete_key == "$accept_key" ]]; then
        _MBX_GHOST_INSTALLED=1
        return 0
    fi
    _mbx_ghost_can_wrap "$delete_key" emacs delete-char || {
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
    # Bind Enter helpers before printables. Wrapping self-insert first and then
    # failing the helper left suffixes visible with stock accept-line (M-044).
    _mbx_ghost_bind_fn emacs "$delete_key" delete-char delete-char || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_bind_fn emacs "$accept_key" accept-line accept-line || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _MBX_GHOST_DELETE_KEYSEQ=$delete_key
    _MBX_GHOST_ACCEPT_KEYSEQ=$accept_key
    _mbx_ghost_bind_self_chars emacs "$chars" || {
        _MBX_GHOST_INSTALLED=1
        return 0
    }
    _mbx_ghost_bind_x emacs '\C-h' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x emacs '\C-?' _mbx_ghost_backspace backward-delete-char || true
    _mbx_ghost_bind_x emacs '\e[C' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x emacs '\C-f' _mbx_ghost_forward forward-char || true
    _mbx_ghost_bind_x emacs '\e[D' _mbx_ghost_backward backward-char || true
    _mbx_ghost_bind_x emacs '\C-b' _mbx_ghost_backward backward-char || true
    _mbx_ghost_bind_x emacs '\eOD' _mbx_ghost_backward backward-char || true
    _mbx_ghost_bind_x emacs '\C-a' _mbx_ghost_beginning beginning-of-line || true
    _mbx_ghost_bind_x emacs '\eOH' _mbx_ghost_beginning beginning-of-line || true
    _mbx_ghost_bind_x emacs '\e[1~' _mbx_ghost_beginning beginning-of-line || true
    _mbx_ghost_bind_x emacs '\e[H' _mbx_ghost_beginning beginning-of-line || true
    _mbx_ghost_bind_x emacs '\C-p' _mbx_ghost_previous_history previous-history || \
        true
    _mbx_ghost_bind_x emacs '\eOA' _mbx_ghost_previous_history previous-history || \
        true
    _mbx_ghost_bind_x emacs '\e[A' _mbx_ghost_previous_history previous-history || \
        true
    _mbx_ghost_bind_x emacs '\C-n' _mbx_ghost_next_history next-history || true
    _mbx_ghost_bind_x emacs '\eOB' _mbx_ghost_next_history next-history || true
    _mbx_ghost_bind_x emacs '\e[B' _mbx_ghost_next_history next-history || true
    _mbx_ghost_bind_x emacs '\eb' _mbx_ghost_backward_word backward-word || true
    _mbx_ghost_bind_x emacs '\e[1;5D' _mbx_ghost_backward_word backward-word || true
    _mbx_ghost_bind_x emacs '\e[5D' _mbx_ghost_backward_word backward-word || true
    _mbx_ghost_bind_x emacs '\ef' _mbx_ghost_forward_word forward-word || true
    _mbx_ghost_bind_x emacs '\e[1;5C' _mbx_ghost_forward_word forward-word || true
    _mbx_ghost_bind_x emacs '\e[5C' _mbx_ghost_forward_word forward-word || true
    if [[ -n $next_key && $next_key != "$delete_key" && $next_key != "$accept_key" ]]; then
        if _mbx_ghost_bind_x emacs "$next_key" _mbx_ghost_cycle_next ''; then
            _MBX_GHOST_CYCLE_BOUND=1
        fi
    fi
    if [[ -n $prev_key && $prev_key != "$delete_key" && $prev_key != "$accept_key" && $prev_key != "$next_key" ]]; then
        _mbx_ghost_bind_x emacs "$prev_key" _mbx_ghost_cycle_prev '' || true
    fi
    _MBX_GHOST_BOUND=1
    if _mbx_ghost_install_vi "$delete_key" "$accept_key" "$next_key" "$prev_key" "$chars"; then
        _MBX_GHOST_VI_BOUND=1
    fi
    _MBX_GHOST_INSTALLED=1
}
