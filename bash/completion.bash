# shellcheck shell=bash
# Stock-completion adapter harness (ADR 0006). Snapshots COMP_* for a wrapped
# -F spec; unknown specs are never replaced. Test fixtures install only when
# MBX_COMP_FIXTURES=1.

_mbx_comp_snapshot() {
    _MBX_COMP_LINE=$COMP_LINE
    _MBX_COMP_POINT=$COMP_POINT
    _MBX_COMP_WORDS=("${COMP_WORDS[@]}")
    _MBX_COMP_CWORD=$COMP_CWORD
    _MBX_COMP_TYPE=${COMP_TYPE-}
    _MBX_COMP_KEY=${COMP_KEY-}
    _MBX_COMP_SNAPPED=1
}

_mbx_comp_sanitize_display() {
    local value=${1-}
    local sanitized= byte
    local code index
    local max=${2:-64}
    local LC_ALL=C

    for ((index = 0; index < ${#value} && index < max; index++)); do
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

_mbx_comp_sanitize_desc() {
    _mbx_comp_sanitize_display "$1" 64
}

_mbx_comp_kind_for_reply() {
    local reply=$1
    local command=${COMP_WORDS[0]:-}
    case $reply in
        --mbx-comp-flag)
            REPLY=flag
            ;;
        mbx_comp_candidate)
            REPLY=word
            ;;
        -*)
            REPLY=flag
            ;;
        *)
            if [[ $command == git || $command == mbx_comp_git ]]; then
                case $reply in
                    */*)
                        REPLY=file
                        ;;
                    *)
                        REPLY=ref
                        ;;
                esac
            else
                REPLY=
            fi
            ;;
    esac
}

_mbx_comp_desc_for_reply() {
    local reply=$1
    case $reply in
        --mbx-comp-flag)
            REPLY='EXTRA fixture flag description'
            ;;
        *)
            REPLY=
            ;;
    esac
}

_mbx_comp_fill_metadata() {
    local i reply kind desc
    local -a kinds=() descs=()

    for ((i = 0; i < ${#COMPREPLY[@]}; i++)); do
        reply=${COMPREPLY[i]}
        kind=${_MBX_COMP_BACKEND_KINDS[i]:-}
        desc=${_MBX_COMP_BACKEND_DESCS[i]:-}
        if [[ -z $kind ]]; then
            _mbx_comp_kind_for_reply "$reply"
            kind=$REPLY
        fi
        if [[ -z $desc ]]; then
            _mbx_comp_desc_for_reply "$reply"
            desc=$REPLY
        fi
        if [[ -n $desc ]]; then
            _mbx_comp_sanitize_desc "$desc"
            desc=$REPLY
        fi
        kinds+=("$kind")
        descs+=("$desc")
    done
    _MBX_COMP_KINDS=("${kinds[@]}")
    _MBX_COMP_DESCS=("${descs[@]}")
}

_mbx_comp_score_reply() {
    local reply=$1
    local word=$2

    if [[ $reply == "$word" ]]; then
        REPLY=300
    elif [[ $reply == "$word"* ]]; then
        REPLY=200
    elif [[ -n $word && $reply == *"$word"* ]]; then
        REPLY=100
    else
        REPLY=0
    fi
}

_mbx_comp_fill_ranking() {
    local cur=${COMP_WORDS[COMP_CWORD]:-}
    local i j n oi oj si sj swap
    local -a scores=() order=()

    n=${#COMPREPLY[@]}
    for ((i = 0; i < n; i++)); do
        if ((i < 64)); then
            _mbx_comp_score_reply "${COMPREPLY[i]}" "$cur"
            scores+=("$REPLY")
        else
            scores+=(0)
        fi
        order+=("$i")
    done
    for ((i = 0; i < n - 1; i++)); do
        for ((j = i + 1; j < n; j++)); do
            oi=${order[i]}
            oj=${order[j]}
            si=${scores[oi]}
            sj=${scores[oj]}
            swap=0
            if ((sj > si)); then
                swap=1
            elif ((sj == si && oj < oi)); then
                swap=1
            fi
            if ((swap)); then
                order[i]=$oj
                order[j]=$oi
            fi
        done
    done
    _MBX_COMP_SCORES=("${scores[@]}")
    _MBX_COMP_ORDER=("${order[@]}")
}

_mbx_comp_fill_ranked_list() {
    local i n
    _MBX_COMP_RANKED_LIST=()
    n=${#_MBX_COMP_ORDER[@]}
    if ((n > 64)); then
        n=64
    fi
    for ((i = 0; i < n; i++)); do
        _MBX_COMP_RANKED_LIST+=("${COMPREPLY[_MBX_COMP_ORDER[i]]}")
    done
    if ((${#_MBX_COMP_RANKED_LIST[@]})); then
        _MBX_COMP_RANKED_REPLY=${_MBX_COMP_RANKED_LIST[0]}
    else
        _MBX_COMP_RANKED_REPLY=
    fi
}

# Remember the completing word's offset so accept/cycle cannot rewrite a later
# prefix-colliding word (M-039). `echo aa` after Tab on `aa` must stay `echo aa`.
_mbx_comp_snapshot_word() {
    local line=${COMP_LINE-}
    local point=${COMP_POINT:-0}
    local start=$point end=$point char
    while ((start > 0)); do
        char=${line:start-1:1}
        [[ $char == [[:space:]] ]] && break
        start=$((start - 1))
    done
    while ((end < ${#line})); do
        char=${line:end:1}
        [[ $char == [[:space:]] ]] && break
        end=$((end + 1))
    done
    _MBX_COMP_SNAP_START=$start
    _MBX_COMP_SNAP_WORD=${line:start:end-start}
}

_mbx_comp_ranked_word_eligible() {
    local current=$1
    local token=${_MBX_COMP_RANKED_REPLY-}
    local start=${_MBX_COMP_WORD_START:-0}
    [[ -n $current && -n $token && $token == "$current"* ]] || return 1
    [[ $start == "${_MBX_COMP_SNAP_START:-}" ]] || return 1
    return 0
}

_mbx_comp_wrap_backend() {
    local backend=$1
    shift
    _mbx_comp_snapshot
    _mbx_comp_snapshot_word
    _MBX_COMP_BACKEND_KINDS=()
    _MBX_COMP_BACKEND_DESCS=()
    "$backend" "$@"
    _MBX_COMP_REPLY_COUNT=${#COMPREPLY[@]}
    _MBX_COMP_LAST_REPLY=${COMPREPLY[0]:-}
    _mbx_comp_fill_metadata
    _mbx_comp_fill_ranking
    _mbx_comp_fill_ranked_list
    _mbx_comp_overlay_snapshot
}

_mbx_comp_overlay_snapshot() {
    [[ ${MBX_COMP_OVERLAY:-} == 1 ]] || return 0
    local i idx
    _MBX_COMP_OVERLAY_CANDIDATES=()
    _MBX_COMP_OVERLAY_KINDS=()
    _MBX_COMP_OVERLAY_DESCS=()
    for ((i = 0; i < ${#_MBX_COMP_RANKED_LIST[@]} && i < 8; i++)); do
        _MBX_COMP_OVERLAY_CANDIDATES+=("${_MBX_COMP_RANKED_LIST[i]}")
        idx=${_MBX_COMP_ORDER[i]:-0}
        _MBX_COMP_OVERLAY_KINDS+=("${_MBX_COMP_KINDS[idx]:-}")
        _MBX_COMP_OVERLAY_DESCS+=("${_MBX_COMP_DESCS[idx]:-}")
    done
    _MBX_COMP_OVERLAY_INDEX=0
# How many candidates the last draw actually put on screen. The overlay caps
# its draw at what the terminal can hold, so this can be fewer than the
# snapshot holds; navigation and acceptance are bounded by it so neither can
# reach a row the user cannot see (M-065 follow-up).
_MBX_COMP_OVERLAY_SHOWN=0
}

_mbx_comp_identifier_ok() {
    [[ $1 == [A-Za-z_][A-Za-z0-9_]* ]]
}

_mbx_comp_f_backend_from_spec() {
    local spec=$1
    local -a words
    local i word
    REPLY=
    read -r -a words <<<"$spec"
    for word in "${words[@]}"; do
        case $word in
            -C|-W|-A|-G)
                return 1
                ;;
        esac
    done
    for ((i = 0; i < ${#words[@]}; i++)); do
        if [[ ${words[i]} == -F ]]; then
            REPLY=${words[i + 1]:-}
            _mbx_comp_identifier_ok "$REPLY" || return 1
            return 0
        fi
    done
    return 1
}

_mbx_comp_unquote_word() {
    local v=$1
    if ((${#v} >= 2)); then
        if [[ $v == \'*\' ]]; then
            v=${v:1:${#v}-2}
        elif [[ $v == \"*\" ]]; then
            v=${v:1:${#v}-2}
        fi
    fi
    REPLY=$v
}

_mbx_comp_options_from_spec() {
    local spec=$1
    local -a words
    local i
    _MBX_COMP_SPEC_OPTS=()
    read -r -a words <<<"$spec"
    for ((i = 0; i < ${#words[@]}; i++)); do
        case ${words[i]} in
            -o | -P | -S | -X)
                if [[ -n ${words[i + 1]:-} ]]; then
                    _mbx_comp_unquote_word "${words[i + 1]}"
                    _MBX_COMP_SPEC_OPTS+=("${words[i]}" "$REPLY")
                fi
                ;;
        esac
    done
}

_mbx_comp_existing_adapter() {
    local command=${COMP_WORDS[0]}
    local backend_var=_MBX_COMP_EXISTING_BACKEND_${command}
    local backend=${!backend_var}
    [[ -n $backend ]] || return 1
    _mbx_comp_wrap_backend "$backend" "$@"
}

_mbx_comp_wrap_existing_f() {
    local command=$1
    local spec backend
    _mbx_comp_identifier_ok "$command" || return 1
    spec=$(complete -p -- "$command" 2>/dev/null) || return 1
    if [[ $spec == *_mbx_comp_*_adapter* ]]; then
        return 0
    fi
    _mbx_comp_f_backend_from_spec "$spec" || return 1
    backend=$REPLY
    declare -F "$backend" >/dev/null 2>&1 || return 1
    printf -v "_MBX_COMP_EXISTING_BACKEND_${command}" '%s' "$backend"
    _mbx_comp_options_from_spec "$spec"
    complete "${_MBX_COMP_SPEC_OPTS[@]}" -F _mbx_comp_existing_adapter -- "$command"
}

_mbx_comp_probe_backend() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    if [[ $cur == mbx_co* ]]; then
        COMPREPLY=(mbx_comp_candidate)
    fi
}

_mbx_comp_probe_adapter() {
    _mbx_comp_wrap_backend _mbx_comp_probe_backend "$@"
}

_mbx_comp_flag_backend() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    if [[ $cur == --mbx-co* ]]; then
        COMPREPLY=(--mbx-comp-flag)
    fi
}

_mbx_comp_flag_nospace_backend() {
    _mbx_comp_flag_backend "$@"
    compopt -o nospace 2>/dev/null || true
}

_mbx_comp_flag_adapter() {
    _mbx_comp_wrap_backend _mbx_comp_flag_backend "$@"
}

_mbx_comp_flag_nospace_adapter() {
    _mbx_comp_wrap_backend _mbx_comp_flag_nospace_backend "$@"
}

_MBX_COMP_ACCEPT_DEFAULT_KEYSEQ='\C-x\C-a'
_MBX_COMP_CYCLE_NEXT_DEFAULT_KEYSEQ='\C-xn'
_MBX_COMP_CYCLE_PREV_DEFAULT_KEYSEQ='\C-xp'
_MBX_COMP_OVERLAY_DEFAULT_KEYSEQ='\C-x\C-o'
_MBX_COMP_OVERLAY_DISMISS_DEFAULT_KEYSEQ='\C-xj'
_MBX_COMP_OVERLAY_VISIBLE=0
_MBX_COMP_OVERLAY_LINES=0
_MBX_COMP_OVERLAY_INDEX=0
_MBX_COMP_OVERLAY_CANDIDATES=()

_mbx_comp_readline_word() {
    local point=${READLINE_POINT:-0}
    local line=${READLINE_LINE-}
    local start=$point end=$point char
    while ((start > 0)); do
        char=${line:start-1:1}
        [[ $char == [[:space:]] ]] && break
        start=$((start - 1))
    done
    while ((end < ${#line})); do
        char=${line:end:1}
        [[ $char == [[:space:]] ]] && break
        end=$((end + 1))
    done
    _MBX_COMP_WORD_START=$start
    _MBX_COMP_WORD_END=$end
    REPLY=${line:start:end-start}
}

_mbx_comp_apply_word_token() {
    local token=$1
    local line=${READLINE_LINE-}
    local start=${_MBX_COMP_WORD_START:-0}
    local end=${_MBX_COMP_WORD_END:-0}
    READLINE_LINE=${line:0:start}${token}${line:end}
    READLINE_POINT=$((start + ${#token}))
}

_mbx_comp_accept_ranked() {
    local token=${_MBX_COMP_RANKED_REPLY-}
    local current
    if [[ ${_MBX_COMP_OVERLAY_VISIBLE:-0} == 1 ]]; then
        # Only a row the overlay actually drew may be accepted. Nothing should
        # be insertable that the user never had on screen to choose.
        local shown=${_MBX_COMP_OVERLAY_SHOWN:-0}
        if ((shown > 0 && _MBX_COMP_OVERLAY_INDEX < shown)); then
            token=${_MBX_COMP_OVERLAY_CANDIDATES[_MBX_COMP_OVERLAY_INDEX]:-}
        else
            token=
        fi
        _mbx_comp_overlay_dismiss
    fi
    [[ -n $token ]] || return 0
    _mbx_comp_readline_word
    current=$REPLY
    # Replace the current word only when it is a non-empty prefix of the ranked
    # candidate at the snapshotted completion offset. That accepts `aa` →
    # `aaflag` and refuses a stale snapshot on `ok` or a later `echo aa`.
    _mbx_comp_ranked_word_eligible "$current" || return 0
    _mbx_comp_apply_word_token "$token"
}

_mbx_comp_cycle_ranked() {
    local direction=$1
    local token=${_MBX_COMP_RANKED_REPLY-}
    local n=${#_MBX_COMP_RANKED_LIST[@]}
    local current first
    [[ -n $token && $n -gt 0 ]] || return 0
    _mbx_comp_readline_word
    current=$REPLY
    [[ -n $current ]] || return 0
    _mbx_comp_ranked_word_eligible "$current" || return 0
    if [[ $current != "$token" ]]; then
        _mbx_comp_apply_word_token "$token"
        return 0
    fi
    ((n >= 2)) || return 0
    if [[ $direction == prev ]]; then
        first=${_MBX_COMP_RANKED_LIST[n - 1]}
        _MBX_COMP_RANKED_LIST=("$first" "${_MBX_COMP_RANKED_LIST[@]:0:n-1}")
    else
        first=${_MBX_COMP_RANKED_LIST[0]}
        _MBX_COMP_RANKED_LIST=("${_MBX_COMP_RANKED_LIST[@]:1}" "$first")
    fi
    token=${_MBX_COMP_RANKED_LIST[0]}
    _MBX_COMP_RANKED_REPLY=$token
    _mbx_comp_apply_word_token "$token"
}

_mbx_comp_cycle_prev() {
    if [[ ${_MBX_COMP_OVERLAY_VISIBLE:-0} == 1 ]]; then
        # Wrap within the rows on screen, not the whole snapshot.
        local n=${_MBX_COMP_OVERLAY_SHOWN:-0}
        ((n > 0)) || n=${#_MBX_COMP_OVERLAY_CANDIDATES[@]}
        ((n > 0)) || return 0
        _MBX_COMP_OVERLAY_INDEX=$(( (_MBX_COMP_OVERLAY_INDEX + n - 1) % n ))
        _mbx_comp_overlay_refresh
        return
    fi
    _mbx_comp_cycle_ranked prev
}

_mbx_comp_overlay_have_tty() {
    [[ -t 1 ]] && [[ -w /dev/tty ]]
}

# Make room for `count` rows below the cursor *before* anything saves the
# cursor position, and return to the starting cell.
#
# `\e7` (DECSC) records an absolute screen position. The overlay used to save
# first and then draw, so whenever the draw itself scrolled the screen the
# saved position no longer referred to the prompt — every row had shifted up —
# and the `\e8` + `\e[J` on dismiss erased from the wrong origin, destroying
# the prompt and the scrollback above it (M-065).
#
# Scrolling here instead inverts that: `\eD` (IND) moves down one row and
# scrolls at the bottom margin, so after `count` of them the screen has
# already absorbed whatever scroll the draw was going to cause. Moving back up
# `count` rows lands on the prompt's row wherever it now is — if the screen
# scrolled by `s`, the cursor is at `L - count` and the prompt moved to
# `R - s`, and those are the same row. A `\e7` taken after this cannot be
# invalidated, because the draw that follows fits in rows that already exist.
#
# IND, not `\n`: IND leaves the column alone. `\n` would return the cursor to
# column 0, so the saved position would be the start of the prompt line rather
# than the user's cursor within it, and the dismissing `\e[J` would then erase
# the prompt text itself — trading a scrollback bug for a worse one.
# How many overlay rows this terminal can show without scrolling the prompt
# off the top.
#
# Reserving rows keeps the saved cursor valid (see `_mbx_comp_overlay_reserve`)
# but does not stop the reservation itself from scrolling the prompt away
# entirely: eight rows do not fit under a prompt on a six-row terminal, and
# drawing them anyway leaves a screen of candidates and no prompt. If the draw
# scrolls by `s`, the prompt lands on row `L - k` for `k` drawn rows, so
# `k <= L - 2` keeps both the prompt and one line of context on screen.
#
# `LINES` is maintained by Bash for an interactive shell (`checkwinsize`, on by
# default) and re-read on SIGWINCH, so it tracks a resize. A missing or
# nonsensical value falls back to the conventional 24 rather than guessing
# zero, which would silently disable the overlay.
_mbx_comp_overlay_capacity() {
    local rows=${LINES:-}

    [[ $rows =~ ^[0-9]+$ ]] && ((rows > 0)) || rows=24
    REPLY=$((rows - 2))
    ((REPLY > 0)) || REPLY=0
}

_mbx_comp_overlay_reserve() {
    local count=${1:-0}
    local index pad=

    ((count > 0)) || return 0
    for ((index = 0; index < count; index++)); do
        pad+=$'\eD'
    done
    printf '%s\e[%dA' "$pad" "$count" >/dev/tty 2>/dev/null || true
}

_mbx_comp_overlay_clear() {
    local lines=${_MBX_COMP_OVERLAY_LINES:-0}
    [[ $lines -gt 0 ]] || {
        _MBX_COMP_OVERLAY_VISIBLE=0
        _MBX_COMP_OVERLAY_SHOWN=0
        return 0
    }
    if _mbx_comp_overlay_have_tty; then
        printf '\e[J' >/dev/tty 2>/dev/null || true
    fi
    _MBX_COMP_OVERLAY_LINES=0
    _MBX_COMP_OVERLAY_VISIBLE=0
    _MBX_COMP_OVERLAY_SHOWN=0
}

_mbx_comp_overlay_refresh() {
    local i n=${#_MBX_COMP_OVERLAY_CANDIDATES[@]}
    local idx=${_MBX_COMP_OVERLAY_INDEX:-0}
    local kind desc candidate
    [[ ${MBX_COMP_OVERLAY:-} == 1 ]] || return 0
    ((n > 0)) || {
        _mbx_comp_overlay_clear
        return 0
    }
    _mbx_comp_overlay_clear
    ((idx >= n)) && idx=$((n - 1))

    # Decide how many rows this draw covers *before* branching on the tty, so
    # `_MBX_COMP_OVERLAY_SHOWN` means the same thing on every path. Navigation
    # and acceptance are bounded by it, and those must not depend on whether
    # this particular process happens to own a terminal.
    local draw=$((n < 8 ? n : 8))
    _mbx_comp_overlay_capacity
    ((draw <= REPLY)) || draw=$REPLY
    if ((draw <= 0)); then
        _MBX_COMP_OVERLAY_VISIBLE=0
        _MBX_COMP_OVERLAY_SHOWN=0
        return 0
    fi
    # The selection must stay on a row that is actually drawn: capping the
    # draw without capping the index would leave nothing highlighted and let
    # ranked accept insert a candidate the user never saw.
    ((idx < draw)) || idx=$((draw - 1))
    _MBX_COMP_OVERLAY_INDEX=$idx
    _MBX_COMP_OVERLAY_SHOWN=$draw

    if _mbx_comp_overlay_have_tty; then
        _mbx_comp_overlay_reserve "$draw"
        printf '\e7' >/dev/tty 2>/dev/null || true
        for ((i = 0; i < draw; i++)); do
            kind=${_MBX_COMP_OVERLAY_KINDS[i]:-}
            desc=${_MBX_COMP_OVERLAY_DESCS[i]:-}
            _mbx_comp_sanitize_display "${_MBX_COMP_OVERLAY_CANDIDATES[i]}"
            candidate=$REPLY
            if ((i == idx)); then
                printf '\n>\033[1m %s\033[0m' "$candidate" >/dev/tty 2>/dev/null || true
            else
                printf '\n  %s' "$candidate" >/dev/tty 2>/dev/null || true
            fi
            if [[ -n $kind || -n $desc ]]; then
                if [[ -n $kind && -n $desc ]]; then
                    printf ' \033[90m(%s: %s)\033[0m' "$kind" "$desc" >/dev/tty 2>/dev/null || true
                elif [[ -n $kind ]]; then
                    printf ' \033[90m(%s)\033[0m' "$kind" >/dev/tty 2>/dev/null || true
                else
                    printf ' \033[90m(%s)\033[0m' "$desc" >/dev/tty 2>/dev/null || true
                fi
            fi
            _MBX_COMP_OVERLAY_LINES=$((_MBX_COMP_OVERLAY_LINES + 1))
        done
        printf '\e8' >/dev/tty 2>/dev/null || true
    fi
    _MBX_COMP_OVERLAY_VISIBLE=1
}

_mbx_comp_overlay_toggle() {
    [[ ${MBX_COMP_OVERLAY:-} == 1 ]] || return 0
    ((${#_MBX_COMP_OVERLAY_CANDIDATES[@]} > 0)) || return 0
    if [[ ${_MBX_COMP_OVERLAY_VISIBLE:-0} == 1 ]]; then
        _mbx_comp_overlay_clear
        return 0
    fi
    _mbx_comp_overlay_refresh
}

_mbx_comp_overlay_dismiss() {
    _mbx_comp_overlay_clear
}

_mbx_comp_install_overlay() {
    [[ ${_MBX_COMP_OVERLAY_INSTALLED:-0} != 1 ]] || return 0
    if [[ $- != *i* ]]; then
        _MBX_COMP_OVERLAY_INSTALLED=1
        return 0
    fi
    [[ ${MBX_COMP_OVERLAY:-} == 1 ]] || {
        _MBX_COMP_OVERLAY_INSTALLED=1
        return 0
    }
    local keyseq=${MBX_COMP_OVERLAY_KEYSEQ:-$_MBX_COMP_OVERLAY_DEFAULT_KEYSEQ}
    local override=${MBX_COMP_OVERLAY_OVERRIDE:-0}
    _MBX_COMP_OVERLAY_BOUND=0
    _MBX_COMP_OVERLAY_VI_INSERT_BOUND=0
    _MBX_COMP_OVERLAY_KEYSEQ_ACTIVE=$keyseq
    local dismiss_keyseq=${MBX_COMP_OVERLAY_DISMISS_KEYSEQ:-$_MBX_COMP_OVERLAY_DISMISS_DEFAULT_KEYSEQ}
    _MBX_COMP_OVERLAY_DISMISS_KEYSEQ_ACTIVE=$dismiss_keyseq
    if _mbx_comp_install_bind_keymap emacs "$keyseq" _mbx_comp_overlay_toggle "$override"; then
        _MBX_COMP_OVERLAY_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap vi-insert "$keyseq" _mbx_comp_overlay_toggle "$override"; then
        _MBX_COMP_OVERLAY_VI_INSERT_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap emacs "$dismiss_keyseq" _mbx_comp_overlay_dismiss \
        "$override"; then
        _MBX_COMP_OVERLAY_DISMISS_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap vi-insert "$dismiss_keyseq" _mbx_comp_overlay_dismiss \
        "$override"; then
        _MBX_COMP_OVERLAY_DISMISS_VI_INSERT_BOUND=1
    fi
    _MBX_COMP_OVERLAY_INSTALLED=1
}

_mbx_comp_cycle_next() {
    if [[ ${_MBX_COMP_OVERLAY_VISIBLE:-0} == 1 ]]; then
        # Wrap within the rows on screen, not the whole snapshot.
        local n=${_MBX_COMP_OVERLAY_SHOWN:-0}
        ((n > 0)) || n=${#_MBX_COMP_OVERLAY_CANDIDATES[@]}
        ((n > 0)) || return 0
        _MBX_COMP_OVERLAY_INDEX=$(( (_MBX_COMP_OVERLAY_INDEX + 1) % n ))
        _mbx_comp_overlay_refresh
        return
    fi
    _mbx_comp_cycle_ranked next
}

_mbx_comp_keyseq_occupied() {
    local keyseq=$1
    local keymap=$2
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    bind -m "$keymap" -p 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    return 1
}

_mbx_comp_install_bind_keymap() {
    local keymap=$1
    local keyseq=$2
    local func=$3
    local override=$4
    if _mbx_comp_keyseq_occupied "$keyseq" "$keymap" && [[ $override != 1 ]]; then
        return 1
    fi
    bind -m "$keymap" -x "\"$keyseq\": $func"
}

_mbx_comp_install_accept() {
    [[ ${_MBX_COMP_ACCEPT_INSTALLED:-0} != 1 ]] || return 0
    if [[ $- != *i* ]]; then
        _MBX_COMP_ACCEPT_INSTALLED=1
        return 0
    fi
    local keyseq=${MBX_COMP_ACCEPT_KEYSEQ:-$_MBX_COMP_ACCEPT_DEFAULT_KEYSEQ}
    local override=${MBX_COMP_ACCEPT_OVERRIDE:-0}
    _MBX_COMP_ACCEPT_BOUND=0
    _MBX_COMP_ACCEPT_VI_INSERT_BOUND=0
    _MBX_COMP_ACCEPT_KEYSEQ_ACTIVE=$keyseq
    if _mbx_comp_install_bind_keymap emacs "$keyseq" _mbx_comp_accept_ranked "$override"; then
        _MBX_COMP_ACCEPT_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap vi-insert "$keyseq" _mbx_comp_accept_ranked "$override"; then
        _MBX_COMP_ACCEPT_VI_INSERT_BOUND=1
    fi
    _MBX_COMP_ACCEPT_INSTALLED=1
}

_mbx_comp_install_cycle() {
    [[ ${_MBX_COMP_CYCLE_INSTALLED:-0} != 1 ]] || return 0
    if [[ $- != *i* ]]; then
        _MBX_COMP_CYCLE_INSTALLED=1
        return 0
    fi
    local next_keyseq=${MBX_COMP_CYCLE_NEXT_KEYSEQ:-$_MBX_COMP_CYCLE_NEXT_DEFAULT_KEYSEQ}
    local prev_keyseq=${MBX_COMP_CYCLE_PREV_KEYSEQ:-$_MBX_COMP_CYCLE_PREV_DEFAULT_KEYSEQ}
    local override=${MBX_COMP_CYCLE_OVERRIDE:-0}
    _MBX_COMP_CYCLE_NEXT_BOUND=0
    _MBX_COMP_CYCLE_NEXT_VI_INSERT_BOUND=0
    _MBX_COMP_CYCLE_PREV_BOUND=0
    _MBX_COMP_CYCLE_PREV_VI_INSERT_BOUND=0
    _MBX_COMP_CYCLE_NEXT_KEYSEQ_ACTIVE=$next_keyseq
    _MBX_COMP_CYCLE_PREV_KEYSEQ_ACTIVE=$prev_keyseq
    if _mbx_comp_install_bind_keymap emacs "$next_keyseq" _mbx_comp_cycle_next "$override"; then
        _MBX_COMP_CYCLE_NEXT_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap vi-insert "$next_keyseq" _mbx_comp_cycle_next "$override"; then
        _MBX_COMP_CYCLE_NEXT_VI_INSERT_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap emacs "$prev_keyseq" _mbx_comp_cycle_prev "$override"; then
        _MBX_COMP_CYCLE_PREV_BOUND=1
    fi
    if _mbx_comp_install_bind_keymap vi-insert "$prev_keyseq" _mbx_comp_cycle_prev "$override"; then
        _MBX_COMP_CYCLE_PREV_VI_INSERT_BOUND=1
    fi
    _MBX_COMP_CYCLE_INSTALLED=1
}

_mbx_comp_rank_backend() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    if [[ $cur == aa* ]]; then
        COMPREPLY=(zzflag aaflag)
    fi
}

_mbx_comp_rank_adapter() {
    _mbx_comp_wrap_backend _mbx_comp_rank_backend "$@"
}

_mbx_comp_git_backend() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    if [[ $cur == aa* ]]; then
        COMPREPLY=(zzref aaref --git-flag src/lib.rs)
    fi
}

_mbx_comp_git_adapter() {
    _mbx_comp_wrap_backend _mbx_comp_git_backend "$@"
}

_mbx_comp_install_probe() {
    [[ ${_MBX_COMP_PROBE_INSTALLED:-0} == 1 ]] || {
        if ! declare -F mbx_comp_probe >/dev/null 2>&1; then
            mbx_comp_probe() { :; }
        fi
        complete -o bashdefault -o default -F _mbx_comp_probe_adapter mbx_comp_probe
        _MBX_COMP_PROBE_INSTALLED=1
    }
}

_mbx_comp_install_flag() {
    [[ ${_MBX_COMP_FLAG_INSTALLED:-0} == 1 ]] || {
        if ! declare -F mbx_comp_flag >/dev/null 2>&1; then
            mbx_comp_flag() { printf 'GOT:%s|\n' "$*"; }
        fi
        if ! declare -F mbx_comp_flag_nospace >/dev/null 2>&1; then
            mbx_comp_flag_nospace() { printf 'GOT:%s|\n' "$*"; }
        fi
        complete -o bashdefault -o default -F _mbx_comp_flag_adapter mbx_comp_flag
        complete -o bashdefault -o default -F _mbx_comp_flag_nospace_adapter \
            mbx_comp_flag_nospace
        _MBX_COMP_FLAG_INSTALLED=1
    }
}

_mbx_comp_install_rank() {
    [[ ${_MBX_COMP_RANK_INSTALLED:-0} == 1 ]] || {
        if ! declare -F mbx_comp_rank >/dev/null 2>&1; then
            mbx_comp_rank() { printf 'GOT:%s|\n' "$*"; }
        fi
        complete -o bashdefault -o default -F _mbx_comp_rank_adapter mbx_comp_rank
        _MBX_COMP_RANK_INSTALLED=1
    }
}

_mbx_comp_install_git() {
    [[ ${_MBX_COMP_GIT_INSTALLED:-0} == 1 ]] || {
        if ! declare -F mbx_comp_git >/dev/null 2>&1; then
            mbx_comp_git() { printf 'GOT:%s|\n' "$*"; }
        fi
        complete -o bashdefault -o default -F _mbx_comp_git_adapter mbx_comp_git
        _MBX_COMP_GIT_INSTALLED=1
    }
}

_mbx_comp_command_uses_adapter() {
    local command=$1
    complete -p "$command" 2>/dev/null | grep -Fq '_mbx_comp_probe_adapter'
}

_mbx_comp_command_uses_flag_adapter() {
    local command=$1
    complete -p "$command" 2>/dev/null | grep -Fq '_mbx_comp_flag'
}

_mbx_comp_wrap_configured() {
    local spec=${MBX_COMP_WRAP-}
    local name rest
    [[ -n $spec ]] || return 0
    rest=$spec
    while [[ -n $rest ]]; do
        if [[ $rest == *[:,]* ]]; then
            name=${rest%%[:,]*}
            rest=${rest#*[:,]}
        else
            name=$rest
            rest=
        fi
        name=${name//[[:space:]]/}
        [[ -n $name ]] || continue
        _mbx_comp_wrap_existing_f "$name" || true
    done
}

_mbx_completion_install() {
    [[ ${_MBX_COMPLETION_INSTALLED:-0} != 1 ]] || return 0
    _mbx_comp_install_accept
    _mbx_comp_install_cycle
    _mbx_comp_install_overlay
    if [[ ${MBX_COMP_FIXTURES:-0} == 1 ]]; then
        _mbx_comp_install_probe
        _mbx_comp_install_flag
        _mbx_comp_install_rank
        _mbx_comp_install_git
    fi
    _mbx_comp_wrap_configured
    _MBX_COMPLETION_INSTALLED=1
}
