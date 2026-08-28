# shellcheck shell=bash
# Environment-to-prompt policy. Transport adapters consume the resulting flags
# rather than re-reading user configuration independently.

_mbx_user_config_path() {
    local path=${MBX_CONFIG-} base
    if [[ -n $path ]]; then
        REPLY=$path
        return 0
    fi
    base=${XDG_CONFIG_HOME:-}
    if [[ $base != /* ]]; then
        if [[ -n ${HOME:-} ]]; then
            base=$HOME/.config
        else
            return 1
        fi
    fi
    REPLY=$base/mbx/config.bash
}

_mbx_load_user_config() {
    local path name i
    local -a keep_names keep_values
    [[ ${_MBX_USER_CONFIG_LOADED:-0} == 1 ]] && return 0
    _MBX_USER_CONFIG_LOADED=1
    _mbx_user_config_path || return 0
    path=$REPLY
    [[ $path == /* ]] || return 0
    [[ -f $path && -r $path ]] || return 0
    keep_names=()
    keep_values=()
    for name in "${!MBX_@}"; do
        keep_names+=("$name")
        keep_values+=("${!name}")
    done
    # shellcheck disable=SC1090
    source "$path"
    if ((${#keep_names[@]} > 0)); then
        for i in "${!keep_names[@]}"; do
            name=${keep_names[i]}
            printf -v "$name" '%s' "${keep_values[i]}"
            export "$name"
        done
    fi
}

mbx_status() {
    local path=-
    _mbx_user_config_path && path=$REPLY
    printf 'config: %s\n' "$path"
    printf 'helper: %s\n' "${MBX_BIN:-unset}"
    printf 'history: %s\n' "${MBX_HISTORY:-off}"
    printf 'ghost: %s (bound=%s)\n' "${MBX_GHOST:-off}" "${_MBX_GHOST_BOUND:-0}"
    printf 'highlight: %s (bound=%s)\n' "${MBX_HIGHLIGHT:-off}" "${_MBX_HIGHLIGHT_BOUND:-0}"
    printf 'overlay: %s (bound=%s)\n' "${MBX_COMP_OVERLAY:-off}" "${_MBX_COMP_OVERLAY_BOUND:-0}"
    printf 'wrap: %s\n' "${MBX_COMP_WRAP:-off}"
    printf 'search: Ctrl-X h  restore: Ctrl-X l  ghost cycle: Ctrl-X Ctrl-N/P\n'
    printf 'overlay: Ctrl-X Ctrl-O  accept: Ctrl-X Ctrl-A  dismiss: Ctrl-X j\n'
}

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
