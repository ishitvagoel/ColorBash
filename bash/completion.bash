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

_mbx_comp_sanitize_desc() {
    local value=${1-}
    local sanitized= byte
    local code index
    local LC_ALL=C

    for ((index = 0; index < ${#value} && index < 64; index++)); do
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

_mbx_comp_wrap_backend() {
    local backend=$1
    shift
    _mbx_comp_snapshot
    _MBX_COMP_BACKEND_KINDS=()
    _MBX_COMP_BACKEND_DESCS=()
    "$backend" "$@"
    _MBX_COMP_REPLY_COUNT=${#COMPREPLY[@]}
    _MBX_COMP_LAST_REPLY=${COMPREPLY[0]:-}
    _mbx_comp_fill_metadata
    _mbx_comp_fill_ranking
    _mbx_comp_fill_ranked_list
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

_mbx_comp_options_from_spec() {
    local spec=$1
    local -a words
    local i
    _MBX_COMP_SPEC_OPTS=()
    read -r -a words <<<"$spec"
    for ((i = 0; i < ${#words[@]}; i++)); do
        if [[ ${words[i]} == -o && -n ${words[i + 1]:-} ]]; then
            _MBX_COMP_SPEC_OPTS+=(-o "${words[i + 1]}")
        fi
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
    if [[ $spec == *_mbx_comp_existing_adapter* ]]; then
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
    [[ -n $token ]] || return 0
    _mbx_comp_readline_word
    current=$REPLY
    # Replace the current word only when it is a non-empty prefix of the ranked
    # candidate. That accepts `aa` → `aaflag` and refuses a stale snapshot on
    # an unrelated word such as `ok`.
    [[ -n $current && $token == "$current"* ]] || return 0
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
    if [[ $token == "$current"* && $current != "$token" ]]; then
        _mbx_comp_apply_word_token "$token"
        return 0
    fi
    [[ $current == "$token" ]] || return 0
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

_mbx_comp_cycle_next() {
    _mbx_comp_cycle_ranked next
}

_mbx_comp_cycle_prev() {
    _mbx_comp_cycle_ranked prev
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

_mbx_completion_install() {
    [[ ${_MBX_COMPLETION_INSTALLED:-0} != 1 ]] || return 0
    _mbx_comp_install_accept
    _mbx_comp_install_cycle
    if [[ ${MBX_COMP_FIXTURES:-0} == 1 ]]; then
        _mbx_comp_install_probe
        _mbx_comp_install_flag
        _mbx_comp_install_rank
        _mbx_comp_install_git
    fi
    _MBX_COMPLETION_INSTALLED=1
}
