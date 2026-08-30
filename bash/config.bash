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
    local path=- helper persist=no
    _mbx_user_config_path && path=$REPLY
    helper=${MBX_BIN:-unset}
    if [[ $helper != unset ]]; then
        if [[ -x $helper ]]; then
            helper="$helper (executable)"
        else
            helper="$helper (missing)"
        fi
    fi
    if [[ -n ${HOME:-} && -f ${HOME}/.bashrc ]] && grep -Fq '# >>> mbx begin' "$HOME/.bashrc"; then
        persist=yes
    fi
    printf 'config: %s\n' "$path"
    printf 'helper: %s\n' "$helper"
    printf 'persist-bashrc: %s\n' "$persist"
    printf 'history: %s\n' "${MBX_HISTORY:-off}"
    printf 'ghost: %s (bound=%s)\n' "${MBX_GHOST:-off}" "${_MBX_GHOST_BOUND:-0}"
    printf 'highlight: %s (bound=%s)\n' "${MBX_HIGHLIGHT:-off}" "${_MBX_HIGHLIGHT_BOUND:-0}"
    printf 'overlay: %s (bound=%s)\n' "${MBX_COMP_OVERLAY:-off}" "${_MBX_COMP_OVERLAY_BOUND:-0}"
    printf 'wrap: %s\n' "${MBX_COMP_WRAP:-off}"
    printf 'duration: %s\n' "${MBX_ENABLE_DURATION_TIMING:-off}"
    printf 'ipc: %s\n' "${MBX_IPC_MODE:-auto}"
    printf 'search: Ctrl-X h  restore: Ctrl-X l  ghost cycle: Ctrl-X Ctrl-N/P\n'
    printf 'overlay: Ctrl-X Ctrl-O  accept: Ctrl-X Ctrl-A  dismiss: Ctrl-X j\n'
    printf 'configure: mbx_configure   or: bash scripts/configure.bash\n'
    printf 'diagnose: mbx_doctor\n'
}

_mbx_doctor_line() {
    local level=$1 message=$2 fix=${3-}
    case $level in
        ok) printf '  [OK]   %s\n' "$message" ;;
        warn)
            printf '  [WARN] %s\n' "$message"
            _MBX_DOCTOR_WARN=$((_MBX_DOCTOR_WARN + 1))
            ;;
        fail)
            printf '  [FAIL] %s\n' "$message"
            _MBX_DOCTOR_FAIL=$((_MBX_DOCTOR_FAIL + 1))
            ;;
    esac
    [[ -n $fix ]] && printf '         fix: %s\n' "$fix"
}

# Diagnostic report (brief §41): reports and, unlike mbx_status, explains
# every failure with a concrete next step. Read-only; never writes config,
# ~/.bashrc, or the history store. Exit status is nonzero only on [FAIL].
mbx_doctor() {
    local _MBX_DOCTOR_WARN=0 _MBX_DOCTOR_FAIL=0
    local config_path=- version handshake

    printf 'mbx doctor\n\nShell\n'
    if ((${BASH_VERSINFO[0]:-0} >= 5)); then
        _mbx_doctor_line ok "Bash $BASH_VERSION"
    else
        _mbx_doctor_line fail "Bash $BASH_VERSION is older than the supported 5.x line" \
            'upgrade Bash; behavior below 5.x is unsupported'
    fi
    if [[ $- == *i* ]]; then
        _mbx_doctor_line ok 'shell is interactive'
    else
        _mbx_doctor_line fail 'shell is not interactive; MBX only activates in interactive Bash' \
            'run this from an interactive shell, not a script or bash -c'
    fi
    if [[ -t 1 ]]; then
        _mbx_doctor_line ok 'stdout is a tty'
    else
        _mbx_doctor_line warn \
            'stdout is not a tty; self-insert wrapping (ghost/highlight) needs a real terminal, a piped shell is not PTY evidence' \
            'run this in a real terminal, not a pipe, redirect, or command substitution'
    fi

    printf '\nTerminal capability\n'
    printf '  TERM=%s COLORTERM=%s LANG=%s MBX_COLOR=%s\n' \
        "${TERM:-unset}" "${COLORTERM:-unset}" "${LANG:-unset}" "${MBX_COLOR:-auto}"
    _mbx_color_capable
    if [[ $REPLY == 1 ]]; then
        _mbx_doctor_line ok 'color is capable'
    else
        _mbx_doctor_line warn \
            'color is disabled (no tty, TERM=dumb, NO_COLOR set, or MBX_COLOR=never)' \
            'unset NO_COLOR, set MBX_COLOR=auto, or use a real terminal, if this is unintended'
    fi
    case ${LANG:-}${LC_ALL:-} in
        *UTF-8* | *utf8*) _mbx_doctor_line ok 'locale advertises UTF-8' ;;
        *)
            _mbx_doctor_line warn 'locale does not advertise UTF-8; wide/combining glyphs may misalign' \
                'export LANG=<your-locale>.UTF-8'
            ;;
    esac
    case ${MBX_ICONS:-auto} in
        nerd) _mbx_doctor_line ok 'icons: Nerd Font glyphs requested (MBX_ICONS=nerd)' ;;
        never | ascii) _mbx_doctor_line ok "icons: ASCII fallback requested (MBX_ICONS=$MBX_ICONS)" ;;
        *)
            _mbx_doctor_line ok \
                'icons: auto (text fallbacks; set MBX_ICONS=nerd only if your font has Nerd Font glyphs)'
            ;;
    esac

    printf '\nHelper\n'
    if [[ -n ${MBX_BIN:-} && -x $MBX_BIN ]]; then
        _mbx_doctor_line ok "MBX_BIN=$MBX_BIN is executable"
        if version=$("$MBX_BIN" --version 2>/dev/null); then
            _mbx_doctor_line ok "version: $version"
        else
            _mbx_doctor_line fail "$MBX_BIN --version failed" \
                'rebuild: cargo build --release --workspace from the ColorBash tree'
        fi
        if handshake=$("$MBX_BIN" handshake 2>/dev/null); then
            _mbx_doctor_line ok "live handshake: $handshake"
        else
            _mbx_doctor_line fail "$MBX_BIN handshake failed" \
                'rebuild the helper, or confirm MBX_BIN points at a real mbx binary'
        fi
    else
        _mbx_doctor_line fail "MBX_BIN is unset or not executable (${MBX_BIN:-unset})" \
            'cargo build --release --workspace, then export MBX_BIN=<repo>/target/release/mbx, or re-run scripts/install.bash'
    fi
    if [[ ${_MBX_ENGINE_READY:-0} == 1 ]]; then
        _mbx_doctor_line ok "coprocess attached (IPC mode: ${MBX_IPC_MODE:-auto})"
    else
        _mbx_doctor_line warn \
            "no coprocess attached; using the per-call/spawn transport (IPC mode: ${MBX_IPC_MODE:-auto})" \
            'expected under MBX_IPC_MODE=off/per-call or MBX_DISABLE_RENDERER=1; otherwise re-source bash/init.bash'
    fi

    printf '\nConfiguration\n'
    _mbx_user_config_path && config_path=$REPLY
    if [[ $config_path == /* && -f $config_path ]]; then
        _mbx_doctor_line ok "config file: $config_path"
    elif [[ $config_path == - ]]; then
        _mbx_doctor_line warn 'no config path could be resolved (HOME unset?)' \
            'export HOME, or set MBX_CONFIG to an absolute path'
    else
        _mbx_doctor_line ok "no config file at $config_path; using environment defaults"
    fi

    printf '\nKeybinding collisions\n'
    # Every chord MBX installs, not only the opt-in ones: the always-on
    # installers (history search and its restore, insert token, ranked accept
    # and cycle) decline an occupied chord exactly as ghost/highlight/overlay
    # do, and each has its own `*_OVERRIDE` escape hatch. Reporting only the
    # three opt-in features left the most common collisions — `\C-xh` and
    # `\C-x\C-y` are popular chords — completely invisible.
    #
    # Field 1 is the env var that gates the feature, or `-` for one that
    # installs whenever the shell is interactive. Field 5 marks the two
    # features whose installer additionally requires PTY evidence, so only
    # those get the tty explanation — attributing a declined chord to "no tty"
    # for a feature that never checked one would send the reader after the
    # wrong cause. Note the installers test *stdin* (`-t 0`), which is what
    # self-insert wrapping needs; this check must match them rather than
    # asking about stdout.
    local -a doctor_features=(
        '-:_MBX_SEARCH_BOUND:MBX_SEARCH_OVERRIDE:history-search insert (Ctrl-X h):0'
        '-:_MBX_SEARCH_RESTORE_BOUND:MBX_SEARCH_RESTORE_OVERRIDE:history-search restore (Ctrl-X l):0'
        '-:_MBX_EDITOR_INSERT_BOUND:MBX_EDITOR_OVERRIDE:insert token (Ctrl-X Ctrl-Y):0'
        '-:_MBX_COMP_ACCEPT_BOUND:MBX_COMP_ACCEPT_OVERRIDE:ranked accept (Ctrl-X Ctrl-A):0'
        '-:_MBX_COMP_CYCLE_NEXT_BOUND:MBX_COMP_CYCLE_OVERRIDE:ranked cycle next (Ctrl-X n):0'
        '-:_MBX_COMP_CYCLE_PREV_BOUND:MBX_COMP_CYCLE_OVERRIDE:ranked cycle previous (Ctrl-X p):0'
        'MBX_GHOST:_MBX_GHOST_BOUND:MBX_GHOST_OVERRIDE:ghost suffix:1'
        'MBX_HIGHLIGHT:_MBX_HIGHLIGHT_BOUND:MBX_HIGHLIGHT_OVERRIDE:syntax highlighting:1'
        'MBX_COMP_OVERLAY:_MBX_COMP_OVERLAY_BOUND:MBX_COMP_OVERLAY_OVERRIDE:completion overlay toggle (Ctrl-X Ctrl-O):0'
        'MBX_COMP_OVERLAY:_MBX_COMP_OVERLAY_DISMISS_BOUND:MBX_COMP_OVERLAY_OVERRIDE:completion overlay dismiss (Ctrl-X j):0'
    )
    local entry env_var bound_var override_var label tty_gated any_checked=0
    for entry in "${doctor_features[@]}"; do
        IFS=: read -r env_var bound_var override_var label tty_gated <<<"$entry"
        [[ $env_var == - || ${!env_var:-0} == 1 ]] || continue
        any_checked=1
        if [[ ${!bound_var:-0} == 1 ]]; then
            _mbx_doctor_line ok "$label: chord bound"
        elif [[ -z ${!bound_var+x} ]]; then
            _mbx_doctor_line warn "$label: its installer has not run in this shell" \
                'source bash/init.bash from an interactive shell'
        elif [[ $env_var == MBX_HIGHLIGHT && ${MBX_GHOST:-0} == 1 ]]; then
            _mbx_doctor_line warn "$label: not bound because MBX_GHOST=1 is also set (mutually exclusive)" ''
        elif [[ $tty_gated == 1 && ! -t 0 ]]; then
            _mbx_doctor_line warn "$label: not bound because stdin is not a tty (see Shell above)" \
                'run this in a real terminal; self-insert wrapping needs PTY evidence'
        else
            _mbx_doctor_line warn "$label: its chord was already bound, so MBX left it alone" \
                "export $override_var=1 before sourcing init.bash to force it"
        fi
    done
    ((any_checked)) || _mbx_doctor_line ok 'no MBX keystroke feature is installed'
    if [[ ${MBX_GHOST:-0} == 1 && ${MBX_HIGHLIGHT:-0} == 1 ]]; then
        _mbx_doctor_line fail 'MBX_GHOST=1 and MBX_HIGHLIGHT=1 are both set; they are mutually exclusive' \
            'disable one of them (highlight install skips while ghost is enabled)'
    fi

    printf '\nHistory store\n'
    if [[ ${MBX_HISTORY:-0} == 1 ]]; then
        if [[ -n ${MBX_BIN:-} && -x $MBX_BIN ]]; then
            local store_path store_count store_mode
            if store_path=$("$MBX_BIN" history path 2>/dev/null); then
                _mbx_doctor_line ok "store: $store_path"
                if [[ -f $store_path ]]; then
                    store_mode=$(stat -c '%a' "$store_path" 2>/dev/null || \
                        stat -f '%Lp' "$store_path" 2>/dev/null)
                    if [[ $store_mode == 600 ]]; then
                        _mbx_doctor_line ok 'store file mode: 0600'
                    else
                        _mbx_doctor_line warn "store file mode is ${store_mode:-unknown}, expected 0600" \
                            "chmod 600 $store_path"
                    fi
                fi
                # A store whose path resolves but whose row count does not is
                # a store the shell cannot actually use — corrupt, locked, or
                # unreadable. Reporting the path and then silently omitting the
                # count would let doctor finish with zero failures while
                # history capture is broken, which is the opposite of what this
                # command is for.
                if store_count=$("$MBX_BIN" history count 2>/dev/null); then
                    _mbx_doctor_line ok "rows: $store_count"
                else
                    _mbx_doctor_line fail \
                        'the history store exists but could not be read (corrupt, locked, or unreadable)' \
                        "check permissions on $store_path, or move it aside to let MBX recreate it"
                fi
            else
                _mbx_doctor_line fail 'could not resolve the history store path' \
                    'confirm MBX_BIN is executable and MBX_HISTORY=1'
            fi
        fi
    else
        _mbx_doctor_line ok 'history capture is off (MBX_HISTORY unset or 0)'
    fi

    printf '\n%d warning(s), %d failure(s)\n' "$_MBX_DOCTOR_WARN" "$_MBX_DOCTOR_FAIL"
    ((_MBX_DOCTOR_FAIL == 0))
}

mbx_configure() {
    local arg from_config=0
    if [[ -n ${_MBX_ROOT:-} && -f "$_MBX_ROOT/scripts/configure.bash" ]]; then
        for arg in "$@"; do
            if [[ $arg == --from-config ]]; then
                from_config=1
                break
            fi
        done
        if ((from_config == 0)); then
            bash "$_MBX_ROOT/scripts/configure.bash" --from-config "$@"
        else
            bash "$_MBX_ROOT/scripts/configure.bash" "$@"
        fi
        return
    fi
    printf 'mbx_configure: run bash scripts/configure.bash from the ColorBash tree\n' >&2
    return 1
}

# Single source of truth for "can this session show color": stdout is a tty,
# TERM is not dumb, and neither NO_COLOR nor MBX_COLOR=never is set. Bash is
# the only side of the coprocess boundary that can see its own controlling
# terminal, so every adapter that needs a color decision (prompt flags,
# highlight's HIGHLIGHT frame and CLI --color fallback) must get it from here
# rather than asking the helper to inspect its own stdout, which is a pipe in
# every IPC path and therefore always reports non-terminal (M-062).
_mbx_color_capable() {
    if [[ ! -t 1 || ${TERM:-dumb} == dumb || -n ${NO_COLOR+x} || ${MBX_COLOR:-auto} == never ]]; then
        REPLY=0
    else
        REPLY=1
    fi
}

_mbx_prompt_flags() {
    local flags=0
    _mbx_color_capable
    if [[ $REPLY == 0 ]]; then
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
