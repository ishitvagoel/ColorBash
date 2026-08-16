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

_mbx_comp_wrap_backend() {
    local backend=$1
    shift
    _mbx_comp_snapshot
    "$backend" "$@"
    _MBX_COMP_REPLY_COUNT=${#COMPREPLY[@]}
    _MBX_COMP_LAST_REPLY=${COMPREPLY[0]:-}
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
    if [[ ${MBX_COMP_FIXTURES:-0} == 1 ]]; then
        _mbx_comp_install_probe
        _mbx_comp_install_flag
    fi
    _MBX_COMPLETION_INSTALLED=1
}
