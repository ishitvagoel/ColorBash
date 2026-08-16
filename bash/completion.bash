# shellcheck shell=bash
# Stock-completion adapter harness (ADR 0006). Snapshots COMP_* for a wrapped
# -F spec; unknown specs are never replaced.

_mbx_comp_snapshot() {
    _MBX_COMP_LINE=$COMP_LINE
    _MBX_COMP_POINT=$COMP_POINT
    _MBX_COMP_WORDS=("${COMP_WORDS[@]}")
    _MBX_COMP_CWORD=$COMP_CWORD
    _MBX_COMP_TYPE=${COMP_TYPE-}
    _MBX_COMP_KEY=${COMP_KEY-}
    _MBX_COMP_SNAPPED=1
}

_mbx_comp_wrap_backend() {
    local backend=$1
    shift
    _mbx_comp_snapshot
    "$backend" "$@"
    _MBX_COMP_REPLY_COUNT=${#COMPREPLY[@]}
    _MBX_COMP_LAST_REPLY=${COMPREPLY[0]:-}
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

_mbx_comp_install_probe() {
    [[ ${_MBX_COMP_PROBE_INSTALLED:-0} == 1 ]] || {
        if ! declare -F mbx_comp_probe >/dev/null 2>&1; then
            mbx_comp_probe() { :; }
        fi
        complete -o bashdefault -o default -F _mbx_comp_probe_adapter mbx_comp_probe
        _MBX_COMP_PROBE_INSTALLED=1
    }
}

_mbx_comp_command_uses_adapter() {
    local command=$1
    complete -p "$command" 2>/dev/null | grep -Fq '_mbx_comp_probe_adapter'
}

_mbx_completion_install() {
    [[ ${_MBX_COMPLETION_INSTALLED:-0} != 1 ]] || return 0
    _mbx_comp_install_probe
    _MBX_COMPLETION_INSTALLED=1
}
