#!/usr/bin/env bash
# Interactive MBX option setup. Writes ~/.config/mbx/config.bash.
# Never writes ~/.bashrc unless the persist option is on (menu or bashrc=1).
# Already-set environment variables still win when the config is sourced.
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd -P)
ANSWERS_FILE=
PRESET=
WRITE_BASHRC=0
NO_BUILD=1
FROM_CONFIG=0

usage() {
    cat <<'EOF'
Configure MBX options interactively, or from an answers file.

Usage:
  bash scripts/configure.bash
  bash scripts/configure.bash --preset comfort|highlight|prompt
  bash scripts/configure.bash --from-config --answers FILE
  mbx_configure                  # after source bash/init.bash

--answers FILE   Non-interactive KEY=value lines (tests / scripting)
--from-config    Load the existing config file first (mbx_configure default)
--preset NAME    Apply a named profile after any --from-config load
--bashrc         Default persist-in-bashrc to yes (still confirmed in the menu)
--build          cargo build --release --workspace before writing
--no-build       Skip cargo build (default for this script)

Answers keys (bools are 0 or 1). Unknown keys are rejected.
  preset history ghost highlight overlay wrap color icons disable_git
  production duration search_cwd search_failed exclude ipc renderer
  render_timeout search_timeout highlight_timeout ghost_limit search_limit
  log editor_token bashrc
  plus *_override and *_keyseq keys listed on the Advanced screen.

Ghost and highlight cannot both be on; highlight is forced off if both are 1.
EOF
}

config_path() {
    local base=${XDG_CONFIG_HOME:-}
    if [[ $base != /* ]]; then
        base=${HOME:?HOME is required}/.config
    fi
    printf '%s\n' "$base/mbx/config.bash"
}

declare -A V=()

reset_defaults() {
    V=(
        [history]=0 [ghost]=0 [highlight]=0 [overlay]=0 [wrap]=
        [color]=auto [icons]=auto [disable_git]=0 [production]=0 [duration]=0
        [search_cwd]=1 [search_failed]=0 [exclude]= [ipc]=auto [renderer]=0
        [render_timeout]= [search_timeout]= [highlight_timeout]=
        [ghost_limit]= [search_limit]= [log]= [editor_token]=
        [bashrc]=0
        [ghost_override]=0 [search_override]=0 [search_restore_override]=0
        [editor_override]=0 [comp_accept_override]=0 [comp_cycle_override]=0
        [overlay_override]=0 [highlight_override]=0
        [ghost_delete_keyseq]= [ghost_accept_keyseq]=
        [ghost_next_keyseq]= [ghost_prev_keyseq]=
        [search_keyseq]= [search_restore_keyseq]= [editor_keyseq]=
        [comp_accept_keyseq]= [comp_cycle_next_keyseq]= [comp_cycle_prev_keyseq]=
        [overlay_keyseq]= [overlay_dismiss_keyseq]= [highlight_accept_keyseq]=
    )
}

apply_preset() {
    local name=$1
    local keep_bashrc=${V[bashrc]-0}
    reset_defaults
    V[bashrc]=$keep_bashrc
    case $name in
        comfort)
            V[history]=1
            V[ghost]=1
            V[overlay]=1
            V[wrap]=git
            ;;
        highlight)
            V[history]=1
            V[highlight]=1
            V[overlay]=1
            V[wrap]=git
            ;;
        prompt) ;;
        *)
            printf 'unknown preset: %s\n' "$name" >&2
            return 2
            ;;
    esac
}

on_off() {
    if [[ ${1:-0} == 1 ]]; then
        printf 'on'
    else
        printf 'off'
    fi
}

show() {
    local v=${1-}
    if [[ -z $v ]]; then
        printf '(default)'
    else
        printf '%s' "$v"
    fi
}

normalize() {
    if [[ ${V[ghost]:-0} == 1 && ${V[highlight]:-0} == 1 ]]; then
        V[highlight]=0
        printf 'Note: ghost and highlight cannot combine; highlight left off.\n' >&2
    fi
    if [[ ${V[ghost]:-0} == 1 ]]; then
        V[history]=1
    fi
    if [[ ${V[history]:-0} != 1 ]]; then
        V[ghost]=0
    fi
    V[wrap]=$(sanitize_wrap "${V[wrap]-}")
}

sanitize_wrap() {
    local spec=$1 name rest out=
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
        if [[ ! $name =~ ^[A-Za-z_][A-Za-z0-9_-]*$ ]]; then
            printf 'skipping wrap command %q (not a command name)\n' "$name" >&2
            continue
        fi
        if [[ -n $out ]]; then
            out+=:$name
        else
            out=$name
        fi
    done
    printf '%s' "$out"
}

safe_string() {
    local val=$1
    if [[ $val == *$'\n'* || $val == *$'\r'* || $val == *'$'* || $val == *'`'* ]]; then
        return 1
    fi
    return 0
}

safe_keyseq() {
    local val=$1
    [[ -n $val ]] || return 0
    ((${#val} <= 32)) || return 1
    safe_string "$val" || return 1
    [[ $val == \\* ]] || return 1
}

safe_timeout() {
    local val=$1
    [[ -n $val ]] || return 0
    [[ $val =~ ^[0-9]+(\.[0-9]+)?$ ]]
}

safe_int() {
    local val=$1
    [[ -n $val ]] || return 0
    [[ $val == [0-9]* && $val != *[!0-9]* ]] || return 1
}

set_bool() {
    local key=$1 raw=$2
    case $raw in
        1 | on | yes | y | true) V[$key]=1 ;;
        0 | off | no | n | false) V[$key]=0 ;;
        *)
            printf 'invalid bool for %s: %s\n' "$key" "$raw" >&2
            return 2
            ;;
    esac
}

KNOWN='preset history ghost highlight overlay wrap color icons disable_git production duration search_cwd search_failed exclude ipc renderer render_timeout search_timeout highlight_timeout ghost_limit search_limit log editor_token bashrc ghost_override search_override search_restore_override editor_override comp_accept_override comp_cycle_override overlay_override highlight_override ghost_delete_keyseq ghost_accept_keyseq ghost_next_keyseq ghost_prev_keyseq search_keyseq search_restore_keyseq editor_keyseq comp_accept_keyseq comp_cycle_next_keyseq comp_cycle_prev_keyseq overlay_keyseq overlay_dismiss_keyseq highlight_accept_keyseq'

is_known() {
    [[ " $KNOWN " == *" $1 "* ]]
}

canonicalize_key() {
    case $1 in
        MBX_HISTORY) REPLY=history ;;
        MBX_GHOST) REPLY=ghost ;;
        MBX_HIGHLIGHT) REPLY=highlight ;;
        MBX_COMP_OVERLAY) REPLY=overlay ;;
        MBX_COMP_WRAP) REPLY=wrap ;;
        MBX_COLOR) REPLY=color ;;
        MBX_ICONS) REPLY=icons ;;
        MBX_DISABLE_GIT) REPLY=disable_git ;;
        MBX_PRODUCTION_CONTEXT) REPLY=production ;;
        MBX_ENABLE_DURATION_TIMING) REPLY=duration ;;
        MBX_SEARCH_CWD) REPLY=search_cwd ;;
        MBX_SEARCH_FAILED) REPLY=search_failed ;;
        MBX_HISTORY_EXCLUDE) REPLY=exclude ;;
        MBX_IPC_MODE) REPLY=ipc ;;
        MBX_DISABLE_RENDERER) REPLY=renderer ;;
        MBX_RENDER_TIMEOUT) REPLY=render_timeout ;;
        MBX_SEARCH_TIMEOUT) REPLY=search_timeout ;;
        MBX_HIGHLIGHT_TIMEOUT) REPLY=highlight_timeout ;;
        MBX_GHOST_LIMIT) REPLY=ghost_limit ;;
        MBX_SEARCH_LIMIT) REPLY=search_limit ;;
        MBX_LOG) REPLY=log ;;
        MBX_EDITOR_INSERT_TOKEN) REPLY=editor_token ;;
        MBX_GHOST_OVERRIDE) REPLY=ghost_override ;;
        MBX_SEARCH_OVERRIDE) REPLY=search_override ;;
        MBX_SEARCH_RESTORE_OVERRIDE) REPLY=search_restore_override ;;
        MBX_EDITOR_OVERRIDE) REPLY=editor_override ;;
        MBX_COMP_ACCEPT_OVERRIDE) REPLY=comp_accept_override ;;
        MBX_COMP_CYCLE_OVERRIDE) REPLY=comp_cycle_override ;;
        MBX_COMP_OVERLAY_OVERRIDE) REPLY=overlay_override ;;
        MBX_HIGHLIGHT_OVERRIDE) REPLY=highlight_override ;;
        MBX_GHOST_DELETE_KEYSEQ) REPLY=ghost_delete_keyseq ;;
        MBX_GHOST_ACCEPT_KEYSEQ) REPLY=ghost_accept_keyseq ;;
        MBX_GHOST_NEXT_KEYSEQ) REPLY=ghost_next_keyseq ;;
        MBX_GHOST_PREV_KEYSEQ) REPLY=ghost_prev_keyseq ;;
        MBX_SEARCH_KEYSEQ) REPLY=search_keyseq ;;
        MBX_SEARCH_RESTORE_KEYSEQ) REPLY=search_restore_keyseq ;;
        MBX_EDITOR_INSERT_KEYSEQ) REPLY=editor_keyseq ;;
        MBX_COMP_ACCEPT_KEYSEQ) REPLY=comp_accept_keyseq ;;
        MBX_COMP_CYCLE_NEXT_KEYSEQ) REPLY=comp_cycle_next_keyseq ;;
        MBX_COMP_CYCLE_PREV_KEYSEQ) REPLY=comp_cycle_prev_keyseq ;;
        MBX_COMP_OVERLAY_KEYSEQ) REPLY=overlay_keyseq ;;
        MBX_COMP_OVERLAY_DISMISS_KEYSEQ) REPLY=overlay_dismiss_keyseq ;;
        MBX_HIGHLIGHT_ACCEPT_KEYSEQ) REPLY=highlight_accept_keyseq ;;
        *) REPLY=$1 ;;
    esac
}

apply_answer() {
    local key=$1 val=$2
    canonicalize_key "$key"
    key=$REPLY
    is_known "$key" || {
        printf 'unknown answers key: %s\n' "$1" >&2
        return 2
    }
    case $key in
        preset)
            apply_preset "$val"
            ;;
        history | ghost | highlight | overlay | disable_git | production | duration | search_cwd | search_failed | renderer | bashrc | ghost_override | search_override | search_restore_override | editor_override | comp_accept_override | comp_cycle_override | overlay_override | highlight_override)
            set_bool "$key" "$val"
            ;;
        wrap)
            V[wrap]=$val
            ;;
        color)
            case $val in
                auto | never) V[color]=$val ;;
                *)
                    printf 'color must be auto or never\n' >&2
                    return 2
                    ;;
            esac
            ;;
        icons)
            case $val in
                auto | never | ascii | nerd) V[icons]=$val ;;
                *)
                    printf 'icons must be auto, never, ascii, or nerd\n' >&2
                    return 2
                    ;;
            esac
            ;;
        ipc)
            case $val in
                auto | coprocess | per-call | off) V[ipc]=$val ;;
                *)
                    printf 'ipc must be auto, coprocess, per-call, or off\n' >&2
                    return 2
                    ;;
            esac
            ;;
        exclude | log | editor_token)
            safe_string "$val" || {
                printf '%s contains a forbidden character\n' "$key" >&2
                return 2
            }
            V[$key]=$val
            ;;
        render_timeout | search_timeout | highlight_timeout)
            safe_timeout "$val" || {
                printf 'invalid timeout for %s\n' "$key" >&2
                return 2
            }
            V[$key]=$val
            ;;
        ghost_limit | search_limit)
            safe_int "$val" || {
                printf 'invalid integer for %s\n' "$key" >&2
                return 2
            }
            V[$key]=$val
            ;;
        *_keyseq)
            safe_keyseq "$val" || {
                printf 'invalid keyseq for %s\n' "$key" >&2
                return 2
            }
            V[$key]=$val
            ;;
        *)
            printf 'unhandled key: %s\n' "$key" >&2
            return 2
            ;;
    esac
}

load_answers() {
    local file=$1 line key val
    if [[ $file == - ]]; then
        file=/dev/stdin
    elif [[ ! -f $file || ! -r $file ]]; then
        printf 'answers file not readable: %s\n' "$file" >&2
        return 2
    fi
    while IFS= read -r line || [[ -n $line ]]; do
        [[ -z $line || $line == \#* ]] && continue
        [[ $line == *=* ]] || {
            printf 'answers line must be KEY=value: %s\n' "$line" >&2
            return 2
        }
        key=${line%%=*}
        val=${line#*=}
        apply_answer "$key" "$val"
    done <"$file"
}

detect_bashrc_persist() {
    local bashrc=${HOME:-}/.bashrc
    if [[ -n ${HOME:-} && -f $bashrc ]] && grep -Fq '# >>> mbx begin' "$bashrc"; then
        V[bashrc]=1
    fi
}

load_existing_config() {
    local path dump line key val
    path=$(config_path)
    [[ $path == /* && -f $path && -r $path ]] || return 0
    dump=$(bash --noprofile --norc -c '
        # shellcheck disable=SC1090
        source "$1" >/dev/null || exit 1
        for n in \
            MBX_HISTORY MBX_GHOST MBX_HIGHLIGHT MBX_COMP_OVERLAY MBX_COMP_WRAP \
            MBX_COLOR MBX_ICONS MBX_DISABLE_GIT MBX_PRODUCTION_CONTEXT \
            MBX_ENABLE_DURATION_TIMING MBX_SEARCH_CWD MBX_SEARCH_FAILED \
            MBX_HISTORY_EXCLUDE MBX_IPC_MODE MBX_DISABLE_RENDERER \
            MBX_RENDER_TIMEOUT MBX_SEARCH_TIMEOUT MBX_HIGHLIGHT_TIMEOUT \
            MBX_GHOST_LIMIT MBX_SEARCH_LIMIT MBX_LOG MBX_EDITOR_INSERT_TOKEN \
            MBX_GHOST_OVERRIDE MBX_SEARCH_OVERRIDE MBX_SEARCH_RESTORE_OVERRIDE \
            MBX_EDITOR_OVERRIDE MBX_COMP_ACCEPT_OVERRIDE MBX_COMP_CYCLE_OVERRIDE \
            MBX_COMP_OVERLAY_OVERRIDE MBX_HIGHLIGHT_OVERRIDE \
            MBX_GHOST_DELETE_KEYSEQ MBX_GHOST_ACCEPT_KEYSEQ \
            MBX_GHOST_NEXT_KEYSEQ MBX_GHOST_PREV_KEYSEQ \
            MBX_SEARCH_KEYSEQ MBX_SEARCH_RESTORE_KEYSEQ MBX_EDITOR_INSERT_KEYSEQ \
            MBX_COMP_ACCEPT_KEYSEQ MBX_COMP_CYCLE_NEXT_KEYSEQ \
            MBX_COMP_CYCLE_PREV_KEYSEQ MBX_COMP_OVERLAY_KEYSEQ \
            MBX_COMP_OVERLAY_DISMISS_KEYSEQ MBX_HIGHLIGHT_ACCEPT_KEYSEQ
        do
            if [[ ${!n+x} ]]; then
                printf "%s=%s\n" "$n" "${!n}"
            fi
        done
    ' _ "$path") || {
        printf 'warning: could not read existing config %s\n' "$path" >&2
        return 0
    }
    while IFS= read -r line || [[ -n $line ]]; do
        [[ -z $line || $line != *=* ]] && continue
        key=${line%%=*}
        val=${line#*=}
        apply_answer "$key" "$val" || true
    done <<<"$dump"
    detect_bashrc_persist
}

maybe_build() {
    ((NO_BUILD == 0)) || return 0
    command -v cargo >/dev/null 2>&1 || {
        printf 'mbx configure: Rust/Cargo is required (https://rustup.rs).\n' >&2
        return 2
    }
    (
        cd "$ROOT"
        cargo build --release --workspace
    )
}

warn_missing_helper() {
    if [[ -x $ROOT/target/release/mbx || -x $ROOT/target/debug/mbx ]]; then
        return 0
    fi
    printf 'Note: helper binary is not built yet. From the repo:\n' >&2
    printf '  bash %q\n' "$ROOT/scripts/install.bash" >&2
}

print_written_summary() {
    printf '\nSaved:\n'
    printf '  history=%s ghost=%s highlight=%s overlay=%s wrap=%s\n' \
        "${V[history]}" "${V[ghost]}" "${V[highlight]}" "${V[overlay]}" "$(show "${V[wrap]}")"
    printf '  persist-bashrc=%s color=%s icons=%s ipc=%s\n' \
        "${V[bashrc]}" "${V[color]}" "${V[icons]}" "${V[ipc]}"
}

emit_assign() {
    local var=$1 val=$2
    printf '[[ ${%s+x} ]] || export %s=%q\n' "$var" "$var" "$val"
}

write_config() {
    local path=$1
    local dir=${path%/*}
    mkdir -p "$dir"
    chmod 700 "$dir" 2>/dev/null || true
    normalize
    {
        printf '# MBX config generated by scripts/configure.bash\n'
        printf '# Re-run: bash scripts/configure.bash   or: mbx_configure\n'
        printf '# Variables already set in the environment are not overridden.\n\n'
        if [[ ${V[history]} == 1 ]]; then
            emit_assign MBX_HISTORY 1
        else
            printf '# history off\n'
        fi
        if [[ ${V[ghost]} == 1 ]]; then
            emit_assign MBX_GHOST 1
        fi
        if [[ ${V[highlight]} == 1 ]]; then
            emit_assign MBX_HIGHLIGHT 1
        elif [[ ${V[ghost]} == 1 ]]; then
            printf '# Highlight stays off: MBX_HIGHLIGHT=1 cannot combine with ghost.\n'
        fi
        if [[ ${V[overlay]} == 1 ]]; then
            emit_assign MBX_COMP_OVERLAY 1
        fi
        if [[ -n ${V[wrap]} ]]; then
            emit_assign MBX_COMP_WRAP "${V[wrap]}"
        fi
        [[ ${V[color]} == auto ]] || emit_assign MBX_COLOR "${V[color]}"
        [[ ${V[icons]} == auto ]] || emit_assign MBX_ICONS "${V[icons]}"
        [[ ${V[disable_git]} == 1 ]] && emit_assign MBX_DISABLE_GIT 1
        [[ ${V[production]} == 1 ]] && emit_assign MBX_PRODUCTION_CONTEXT 1
        [[ ${V[duration]} == 1 ]] && emit_assign MBX_ENABLE_DURATION_TIMING 1
        if [[ ${V[search_cwd]} == 0 ]]; then
            emit_assign MBX_SEARCH_CWD 0
        fi
        [[ ${V[search_failed]} == 1 ]] && emit_assign MBX_SEARCH_FAILED 1
        [[ -n ${V[exclude]} ]] && emit_assign MBX_HISTORY_EXCLUDE "${V[exclude]}"
        [[ ${V[ipc]} == auto ]] || emit_assign MBX_IPC_MODE "${V[ipc]}"
        [[ ${V[renderer]} == 1 ]] && emit_assign MBX_DISABLE_RENDERER 1
        [[ -n ${V[render_timeout]} ]] && emit_assign MBX_RENDER_TIMEOUT "${V[render_timeout]}"
        [[ -n ${V[search_timeout]} ]] && emit_assign MBX_SEARCH_TIMEOUT "${V[search_timeout]}"
        [[ -n ${V[highlight_timeout]} ]] && emit_assign MBX_HIGHLIGHT_TIMEOUT "${V[highlight_timeout]}"
        [[ -n ${V[ghost_limit]} ]] && emit_assign MBX_GHOST_LIMIT "${V[ghost_limit]}"
        [[ -n ${V[search_limit]} ]] && emit_assign MBX_SEARCH_LIMIT "${V[search_limit]}"
        [[ -n ${V[log]} ]] && emit_assign MBX_LOG "${V[log]}"
        [[ -n ${V[editor_token]} ]] && emit_assign MBX_EDITOR_INSERT_TOKEN "${V[editor_token]}"
        [[ ${V[ghost_override]} == 1 ]] && emit_assign MBX_GHOST_OVERRIDE 1
        [[ ${V[search_override]} == 1 ]] && emit_assign MBX_SEARCH_OVERRIDE 1
        [[ ${V[search_restore_override]} == 1 ]] && emit_assign MBX_SEARCH_RESTORE_OVERRIDE 1
        [[ ${V[editor_override]} == 1 ]] && emit_assign MBX_EDITOR_OVERRIDE 1
        [[ ${V[comp_accept_override]} == 1 ]] && emit_assign MBX_COMP_ACCEPT_OVERRIDE 1
        [[ ${V[comp_cycle_override]} == 1 ]] && emit_assign MBX_COMP_CYCLE_OVERRIDE 1
        [[ ${V[overlay_override]} == 1 ]] && emit_assign MBX_COMP_OVERLAY_OVERRIDE 1
        [[ ${V[highlight_override]} == 1 ]] && emit_assign MBX_HIGHLIGHT_OVERRIDE 1
        [[ -n ${V[ghost_delete_keyseq]} ]] && emit_assign MBX_GHOST_DELETE_KEYSEQ "${V[ghost_delete_keyseq]}"
        [[ -n ${V[ghost_accept_keyseq]} ]] && emit_assign MBX_GHOST_ACCEPT_KEYSEQ "${V[ghost_accept_keyseq]}"
        [[ -n ${V[ghost_next_keyseq]} ]] && emit_assign MBX_GHOST_NEXT_KEYSEQ "${V[ghost_next_keyseq]}"
        [[ -n ${V[ghost_prev_keyseq]} ]] && emit_assign MBX_GHOST_PREV_KEYSEQ "${V[ghost_prev_keyseq]}"
        [[ -n ${V[search_keyseq]} ]] && emit_assign MBX_SEARCH_KEYSEQ "${V[search_keyseq]}"
        [[ -n ${V[search_restore_keyseq]} ]] && emit_assign MBX_SEARCH_RESTORE_KEYSEQ "${V[search_restore_keyseq]}"
        [[ -n ${V[editor_keyseq]} ]] && emit_assign MBX_EDITOR_INSERT_KEYSEQ "${V[editor_keyseq]}"
        [[ -n ${V[comp_accept_keyseq]} ]] && emit_assign MBX_COMP_ACCEPT_KEYSEQ "${V[comp_accept_keyseq]}"
        [[ -n ${V[comp_cycle_next_keyseq]} ]] && emit_assign MBX_COMP_CYCLE_NEXT_KEYSEQ "${V[comp_cycle_next_keyseq]}"
        [[ -n ${V[comp_cycle_prev_keyseq]} ]] && emit_assign MBX_COMP_CYCLE_PREV_KEYSEQ "${V[comp_cycle_prev_keyseq]}"
        [[ -n ${V[overlay_keyseq]} ]] && emit_assign MBX_COMP_OVERLAY_KEYSEQ "${V[overlay_keyseq]}"
        [[ -n ${V[overlay_dismiss_keyseq]} ]] && emit_assign MBX_COMP_OVERLAY_DISMISS_KEYSEQ "${V[overlay_dismiss_keyseq]}"
        [[ -n ${V[highlight_accept_keyseq]} ]] && emit_assign MBX_HIGHLIGHT_ACCEPT_KEYSEQ "${V[highlight_accept_keyseq]}"
    } >"$path"
    chmod 600 "$path" 2>/dev/null || true
}

persist_bashrc() {
    bash "$ROOT/scripts/install.bash" --no-build --bashrc-only
}

print_menu() {
    cat <<EOF

MBX options    config: $(config_path)
  Suggestions never run until you press Enter. History is local SQLite.

  Features
    1) History sidecar           $(on_off "${V[history]}")
    2) Ghost suggestions         $(on_off "${V[ghost]}")   (needs history; not with highlight)
    3) Syntax highlighting       $(on_off "${V[highlight]}")   (not with ghost)
    4) Completion overlay        $(on_off "${V[overlay]}")
    5) Wrap -F completers        $(show "${V[wrap]}")

  Prompt
    6) Color                     ${V[color]}
    7) Icons                     ${V[icons]}
    8) Git segment               $(on_off "$(if [[ ${V[disable_git]} == 1 ]]; then echo 0; else echo 1; fi)")
    9) Production marker         $(on_off "${V[production]}")
   10) Command duration          $(on_off "${V[duration]}")   (needs a free DEBUG trap)

  History extras
   11) Empty-line search: cwd    $(on_off "${V[search_cwd]}")
   12) Prefer failed commands    $(on_off "${V[search_failed]}")
   13) Exclude globs             $(show "${V[exclude]}")

  Shell
   14) Helper IPC                ${V[ipc]}
   15) Persist in ~/.bashrc      $(on_off "${V[bashrc]}")

   a) Advanced (keys, timeouts, overrides)
   w) Write config and exit
   q) Quit without writing
   ?) Help

EOF
}

print_advanced() {
    cat <<EOF

Advanced (empty keeps product default)
   20) Ghost override occupied keys     $(on_off "${V[ghost_override]}")
   21) Search override                  $(on_off "${V[search_override]}")
   22) Restore override                 $(on_off "${V[search_restore_override]}")
   23) Editor override                  $(on_off "${V[editor_override]}")
   24) Overlay override                 $(on_off "${V[overlay_override]}")
   25) Highlight override               $(on_off "${V[highlight_override]}")
   26) Completer accept/cycle override  $(on_off "${V[comp_accept_override]}") / $(on_off "${V[comp_cycle_override]}")
   27) Disable native renderer          $(on_off "${V[renderer]}")
   28) Ghost limit                      $(show "${V[ghost_limit]}")
   29) Search limit                     $(show "${V[search_limit]}")
   30) Render timeout                   $(show "${V[render_timeout]}")
   31) Search timeout                   $(show "${V[search_timeout]}")
   32) Highlight timeout                $(show "${V[highlight_timeout]}")
   33) Editor insert token              $(show "${V[editor_token]}")
   34) Log level                        $(show "${V[log]}")
   35) Search keyseq                    $(show "${V[search_keyseq]}")
   36) Restore keyseq                   $(show "${V[search_restore_keyseq]}")
   37) Overlay keyseq / dismiss         $(show "${V[overlay_keyseq]}") / $(show "${V[overlay_dismiss_keyseq]}")
   38) Ghost cycle next/prev            $(show "${V[ghost_next_keyseq]}") / $(show "${V[ghost_prev_keyseq]}")
   39) Comp accept / cycle n/p          $(show "${V[comp_accept_keyseq]}") / $(show "${V[comp_cycle_next_keyseq]}") / $(show "${V[comp_cycle_prev_keyseq]}")
   40) Editor / highlight accept        $(show "${V[editor_keyseq]}") / $(show "${V[highlight_accept_keyseq]}")
   41) Ghost delete / accept helpers    $(show "${V[ghost_delete_keyseq]}") / $(show "${V[ghost_accept_keyseq]}")
    b) Back
EOF
}

print_help() {
    cat <<'EOF'
Type a number to toggle or edit that option, then w to save.

History records admitted commands in a local SQLite store; it does not rewrite
.bash_history. Ghost shows an insert-only suffix (Right accepts; Enter runs the
typed prefix). Highlight colors the line; Enter still runs plain bytes.
Ghost and highlight cannot both be on.

Tab stays stock Bash. Overlay lists ranked candidates after Tab on a wrapped
-F completer (comfort wraps git). Occupied Readline chords are skipped unless
you turn on the matching override.

Duration timing installs a DEBUG trap; skip it if you already have one.
Persist appends a managed # >>> mbx begin block to ~/.bashrc (idempotent).
EOF
}

ask() {
    local prompt=$1
    local reply=
    if ! read -r -p "$prompt" reply; then
        printf '\nEOF\n' >&2
        return 1
    fi
    REPLY=$reply
}

toggle() {
    local key=$1
    if [[ ${V[$key]} == 1 ]]; then
        V[$key]=0
    else
        V[$key]=1
    fi
}

ask_set() {
    local key=$1 prompt=$2
    ask "$prompt [${V[$key]-}]: " || return 1
    if [[ -n $REPLY ]]; then
        apply_answer "$key" "$REPLY" || printf 'Value not applied.\n'
    fi
}

handle_choice() {
    local choice=$1
    case $choice in
        1)
            toggle history
            [[ ${V[history]} == 1 ]] || V[ghost]=0
            ;;
        2)
            toggle ghost
            [[ ${V[ghost]} == 1 ]] && V[highlight]=0
            ;;
        3)
            toggle highlight
            [[ ${V[highlight]} == 1 ]] && V[ghost]=0
            ;;
        4) toggle overlay ;;
        5) ask_set wrap 'Wrap which -F commands (colon or comma separated)' ;;
        6) ask_set color 'Color (auto|never)' ;;
        7) ask_set icons 'Icons (auto|never|ascii|nerd)' ;;
        8)
            if [[ ${V[disable_git]} == 1 ]]; then
                V[disable_git]=0
            else
                V[disable_git]=1
            fi
            ;;
        9) toggle production ;;
        10)
            toggle duration
            if [[ ${V[duration]} == 1 ]]; then
                printf 'Duration needs a free DEBUG trap. Skip this if you already use DEBUG.\n'
            fi
            ;;
        11) toggle search_cwd ;;
        12) toggle search_failed ;;
        13) ask_set exclude 'Exclude globs (colon-separated, e.g. git *:ssh *)' ;;
        14) ask_set ipc 'Helper IPC (auto|coprocess|per-call|off)' ;;
        15) toggle bashrc ;;
        a | A)
            advanced_loop
            ;;
        '?' | h | H | help)
            print_help
            ;;
        w | W)
            return 99
            ;;
        q | Q)
            printf 'No changes written.\n'
            exit 0
            ;;
        '') ;;
        *)
            printf 'Unknown choice: %s\n' "$choice"
            ;;
    esac
    normalize
}

advanced_loop() {
    local choice
    while true; do
        print_advanced
        ask 'Advanced select: ' || return 1
        choice=$REPLY
        case $choice in
            b | B | '') return 0 ;;
            20) toggle ghost_override ;;
            21) toggle search_override ;;
            22) toggle search_restore_override ;;
            23) toggle editor_override ;;
            24) toggle overlay_override ;;
            25) toggle highlight_override ;;
            26)
                toggle comp_accept_override
                V[comp_cycle_override]=${V[comp_accept_override]}
                ;;
            27) toggle renderer ;;
            28) ask_set ghost_limit 'Ghost prefix-match limit (1-8)' ;;
            29) ask_set search_limit 'Search snapshot limit (max 16)' ;;
            30) ask_set render_timeout 'Render timeout seconds' ;;
            31) ask_set search_timeout 'Search timeout seconds' ;;
            32) ask_set highlight_timeout 'Highlight timeout seconds' ;;
            33) ask_set editor_token 'Editor insert token' ;;
            34) ask_set log 'Log level (trace or empty)' ;;
            35) ask_set search_keyseq 'Search keyseq (example \\C-xh)' ;;
            36) ask_set search_restore_keyseq 'Restore keyseq (example \\C-xl)' ;;
            37)
                ask_set overlay_keyseq 'Overlay keyseq (example \\C-x\\C-o)'
                ask_set overlay_dismiss_keyseq 'Overlay dismiss (example \\C-xj)'
                ;;
            38)
                ask_set ghost_next_keyseq 'Ghost next (example \\C-x\\C-n)'
                ask_set ghost_prev_keyseq 'Ghost prev (example \\C-x\\C-p)'
                ;;
            39)
                ask_set comp_accept_keyseq 'Accept keyseq (example \\C-x\\C-a)'
                ask_set comp_cycle_next_keyseq 'Cycle next (example \\C-xn)'
                ask_set comp_cycle_prev_keyseq 'Cycle prev (example \\C-xp)'
                ;;
            40)
                ask_set editor_keyseq 'Editor insert keyseq (example \\C-x\\C-y)'
                ask_set highlight_accept_keyseq 'Highlight accept helper (example \\C-x\\C-m)'
                ;;
            41)
                ask_set ghost_delete_keyseq 'Ghost delete helper (example \\C-x\\C-d)'
                ask_set ghost_accept_keyseq 'Ghost accept helper (example \\C-x\\C-m)'
                ;;
            *)
                printf 'Unknown advanced choice: %s\n' "$choice"
                ;;
        esac
        normalize
    done
}

opening_choice() {
    local default=1 have_config=0
    local path
    path=$(config_path)
    if [[ -f $path ]]; then
        have_config=1
        default=4
    fi
    cat <<EOF
MBX interactive setup
  Config will be written to $path
  This does not edit ~/.bashrc unless you turn on persist (option 15).

Start from:
  1) Comfort (recommended) — history, ghost, overlay, wrap git
  2) Highlight — history, syntax color, overlay, wrap git
  3) Prompt only
  4) Current config$(if ((have_config == 1)); then printf ' (detected)'; fi)
  5) Empty defaults

EOF
    ask "Choice [$default]: " || return 1
    case ${REPLY:-$default} in
        1) apply_preset comfort ;;
        2) apply_preset highlight ;;
        3) apply_preset prompt ;;
        '' | 4)
            if ((have_config == 1)); then
                :
            else
                apply_preset comfort
            fi
            ;;
        5) reset_defaults ;;
        *)
            printf 'Unknown start choice; using comfort.\n'
            apply_preset comfort
            ;;
    esac
}

menu_loop() {
    local choice rc
    while true; do
        print_menu
        ask 'Select: ' || exit 2
        choice=$REPLY
        rc=0
        handle_choice "$choice" || rc=$?
        if ((rc == 99)); then
            return 0
        fi
        if ((rc != 0)); then
            printf 'Could not apply that change.\n'
        fi
    done
}

finish() {
    local path
    path=$(config_path)
    write_config "$path"
    printf 'Wrote %s\n' "$path"
    print_written_summary
    warn_missing_helper
    if [[ ${V[bashrc]} == 1 ]]; then
        persist_bashrc
    else
        printf 'Did not edit ~/.bashrc. For this shell:\n\n  source %q\n\n' \
            "$ROOT/bash/init.bash"
    fi
    printf 'Reload with exec bash, then run mbx_status.\n'
    printf 'Re-run this tool anytime: bash %q   or: mbx_configure\n' \
        "$ROOT/scripts/configure.bash"
}

while (($#)); do
    case $1 in
        -h | --help)
            usage
            exit 0
            ;;
        --answers)
            ANSWERS_FILE=${2:?--answers needs a file path or -}
            shift 2
            ;;
        --preset)
            PRESET=${2:?--preset needs comfort, highlight, or prompt}
            shift 2
            ;;
        --bashrc)
            WRITE_BASHRC=1
            shift
            ;;
        --from-config)
            FROM_CONFIG=1
            shift
            ;;
        --no-build)
            NO_BUILD=1
            shift
            ;;
        --build)
            NO_BUILD=0
            shift
            ;;
        *)
            printf 'unknown option: %s\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

reset_defaults
if ((FROM_CONFIG == 1)) || [[ -z $PRESET && -z $ANSWERS_FILE && -f "$(config_path)" ]]; then
    load_existing_config
fi
if [[ -n $PRESET ]]; then
    apply_preset "$PRESET"
fi
if ((WRITE_BASHRC == 1)); then
    V[bashrc]=1
fi

maybe_build

if [[ -n $ANSWERS_FILE ]]; then
    load_answers "$ANSWERS_FILE"
    finish
    exit 0
fi

if [[ -z $PRESET ]]; then
    opening_choice
fi
if ((WRITE_BASHRC == 1)); then
    V[bashrc]=1
fi
menu_loop
finish
