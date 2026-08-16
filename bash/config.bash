# shellcheck shell=bash
# Environment-to-prompt policy. Transport adapters consume the resulting flags
# rather than re-reading user configuration independently.

_mbx_prompt_flags() {
    local flags=0
    if [[ ! -t 1 || ${TERM:-dumb} == dumb || -n ${NO_COLOR+x} || ${MBX_COLOR:-auto} == never ]]; then
        ((flags |= _MBX_FLAG_NO_COLOR))
    fi
    if (( (flags & _MBX_FLAG_NO_COLOR) == 0 )); then
        case ${COLORTERM,,} in
            truecolor|24bit) ((flags |= _MBX_FLAG_TRUECOLOR)) ;;
            *)
                case ${TERM:-} in
                    *256color*|xterm-direct) ;;
                    *) ((flags |= _MBX_FLAG_COLOR_16)) ;;
                esac
                ;;
        esac
    fi
    case ${MBX_ICONS:-auto} in
        never|ascii) ((flags |= _MBX_FLAG_ASCII_ICONS)) ;;
        nerd) ((flags |= _MBX_FLAG_NERD_ICONS)) ;;
    esac
    if [[ -n ${SSH_CONNECTION:-} || -n ${SSH_TTY:-} ]]; then
        ((flags |= _MBX_FLAG_SSH))
    fi
    if [[ ${MBX_PRODUCTION_CONTEXT:-0} == 1 ]]; then
        ((flags |= _MBX_FLAG_PRODUCTION))
    fi
    if [[ ${MBX_DISABLE_GIT:-0} == 1 ]]; then
        ((flags |= _MBX_FLAG_DISABLE_GIT))
    fi
    REPLY=$flags
}

_mbx_coprocess_requested() {
    [[ ${MBX_DISABLE_RENDERER:-0} != 1 ]] || return 1
    [[ -x ${MBX_BIN:-} ]] || return 1
    case ${MBX_IPC_MODE:-auto} in
        auto|coprocess) return 0 ;;
        *) return 1 ;;
    esac
}

_mbx_per_call_available() {
    [[ ${MBX_DISABLE_RENDERER:-0} != 1 ]] || return 1
    [[ -x ${MBX_BIN:-} ]] || return 1
    [[ ${MBX_IPC_MODE:-auto} != off ]]
}
