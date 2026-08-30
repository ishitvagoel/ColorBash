# shellcheck shell=bash
# Prompt lifecycle hooks. Existing PROMPT_COMMAND entries are preserved in order.
# An existing DEBUG trap is never replaced; command-duration timing then degrades
# gracefully while status capture and prompt rendering continue to work.

_mbx_now_us() {
    [[ -n ${EPOCHREALTIME:-} ]] || return 1
    local now=$EPOCHREALTIME
    local seconds=${now%.*}
    local fraction=${now#*.}000000
    fraction=${fraction:0:6}
    REPLY=$((10#$seconds * 1000000 + 10#$fraction))
}

_mbx_preexec_hook() {
    if [[ ${_MBX_SKIP_DEBUG_ONCE:-0} == 1 ]]; then
        _MBX_SKIP_DEBUG_ONCE=0
        return 0
    fi
    [[ ${_MBX_AT_PROMPT:-0} == 1 && ${_MBX_IN_PROMPT_CYCLE:-0} == 0 ]] || return 0
    _MBX_AT_PROMPT=0
    if _mbx_now_us; then
        _MBX_COMMAND_STARTED_US=$REPLY
    else
        unset _MBX_COMMAND_STARTED_US
    fi
}

_mbx_capture_status() {
    local status=$?
    local finished_us
    _MBX_IN_PROMPT_CYCLE=1
    _MBX_LAST_STATUS=$status
    _MBX_LAST_DURATION_MS=-
    if [[ -n ${_MBX_COMMAND_STARTED_US:-} ]] && _mbx_now_us; then
        finished_us=$REPLY
        if (( finished_us >= _MBX_COMMAND_STARTED_US )); then
            _MBX_LAST_DURATION_MS=$(((finished_us - _MBX_COMMAND_STARTED_US) / 1000))
        fi
    fi
    unset _MBX_COMMAND_STARTED_US
    if [[ ${_MBX_HISTORY_ENABLED:-0} == 1 ]]; then
        _mbx_history_prompt
    fi
    return "$status"
}

_mbx_render_prompt() {
    local status=${_MBX_LAST_STATUS:-0}
    _mbx_update_prompt "$status" "${_MBX_LAST_DURATION_MS:--}"
    # Ranked-accept/cycle, history-search, and ghost snapshots are valid only
    # until the next prompt (M-039 analog).
    _MBX_COMP_RANKED_REPLY=
    _MBX_COMP_RANKED_LIST=()
    if declare -F _mbx_search_clear >/dev/null 2>&1; then
        _mbx_search_clear
    fi
    if declare -F _mbx_ghost_clear >/dev/null 2>&1; then
        _mbx_ghost_clear
    fi
    _MBX_AT_PROMPT=1
    if [[ $- == *T* ]]; then
        _MBX_SKIP_DEBUG_ONCE=1
    else
        _MBX_SKIP_DEBUG_ONCE=0
    fi
    _MBX_IN_PROMPT_CYCLE=0
    return "$status"
}

_mbx_install_hooks() {
    [[ ${_MBX_HOOKS_INSTALLED:-0} != 1 ]] || return 0
    local -a existing_prompt_commands=()
    local declaration

    declaration=$(declare -p PROMPT_COMMAND 2>/dev/null) || declaration=
    if [[ $declaration == 'declare -a '* ]]; then
        existing_prompt_commands=("${PROMPT_COMMAND[@]}")
    elif [[ -n ${PROMPT_COMMAND:-} ]]; then
        existing_prompt_commands=("$PROMPT_COMMAND")
    fi
    local -a chained=(
        _mbx_capture_status
        "${existing_prompt_commands[@]}"
        _mbx_render_prompt
    )
    # An *array* `PROMPT_COMMAND` is a Bash 5.1 feature. Bash 5.0 treats the
    # variable as a plain string, so an array assignment there leaves the
    # prompt running only element 0: MBX's own `_mbx_render_prompt` never runs
    # (PS1 is never set, the shell keeps its stock prompt) and any
    # pre-existing PROMPT_COMMAND is silently dropped. That made the whole
    # integration a no-op on a Bash release `docs/bash-compatibility.md` names
    # as supported, while destroying another framework's hook on the way
    # (M-076). Join into a single string there instead; 5.1+ keeps the array,
    # which isolates each entry from a syntax error in its neighbours.
    if ((BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1))); then
        PROMPT_COMMAND=("${chained[@]}")
    else
        local joined
        printf -v joined '%s; ' "${chained[@]}"
        # `unset` first: if PROMPT_COMMAND is already an array, a scalar
        # assignment would only replace element 0 and leave the rest in place.
        unset -v PROMPT_COMMAND
        PROMPT_COMMAND=${joined%'; '}
    fi

    # Bash does not expose a pre-existing DEBUG trap from within a sourced file
    # without changing context. Correctness wins: timing is explicit opt-in so
    # the default integration can never replace another framework's trap.
    if [[ ${MBX_ENABLE_DURATION_TIMING:-0} == 1 ]]; then
        trap '_mbx_preexec_hook' DEBUG
        _MBX_DURATION_TIMING=1
    else
        _MBX_DURATION_TIMING=0
    fi
    _MBX_AT_PROMPT=0
    _MBX_IN_PROMPT_CYCLE=0
    _MBX_HOOKS_INSTALLED=1
}
