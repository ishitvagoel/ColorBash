# shellcheck shell=bash
# Source this file near the end of .bashrc: source /path/to/ColorBash/bash/init.bash

[[ $- == *i* ]] || return 0
[[ ${_MBX_INITIALIZED:-0} != 1 ]] || return 0

_MBX_INIT_FILE=${BASH_SOURCE[0]}
_MBX_BASH_DIR=${_MBX_INIT_FILE%/*}
if [[ $_MBX_BASH_DIR == "$_MBX_INIT_FILE" ]]; then
    _MBX_BASH_DIR=.
fi
_MBX_ROOT=$(cd -- "$_MBX_BASH_DIR/.." 2>/dev/null && pwd -P) || return 0

# Keep each concern inspectable; a helper failure must never prevent the fallback.
source "$_MBX_ROOT/bash/protocol.bash" || return 0
source "$_MBX_ROOT/bash/config.bash" || return 0
_mbx_load_user_config || true
source "$_MBX_ROOT/bash/fallback.bash" || return 0
source "$_MBX_ROOT/bash/engine.bash" || return 0
source "$_MBX_ROOT/bash/prompt.bash" || return 0
source "$_MBX_ROOT/bash/hooks.bash" || return 0
source "$_MBX_ROOT/bash/editor.bash" || return 0
source "$_MBX_ROOT/bash/completion.bash" || return 0
source "$_MBX_ROOT/bash/highlight.bash" || return 0
source "$_MBX_ROOT/bash/history.bash" || return 0
source "$_MBX_ROOT/bash/search.bash" || return 0
source "$_MBX_ROOT/bash/ghost.bash" || return 0

if [[ -z ${MBX_BIN:-} ]]; then
    if [[ -x $_MBX_ROOT/target/release/mbx ]]; then
        MBX_BIN=$_MBX_ROOT/target/release/mbx
    else
        MBX_BIN=$_MBX_ROOT/target/debug/mbx
    fi
fi

# ONBD-001: the first source must not be silent. One banner, once per config
# directory, only on a terminal; onboarding may never block or fail the shell.
_mbx_first_run_notice() {
    local path
    [[ -t 1 ]] || return 0
    _mbx_user_config_path || return 0
    path=${REPLY%/*}/first-run-shown
    [[ -e $path ]] && return 0
    printf 'MBX loaded. Type mbx_help for keys, mbx_status for a summary.\n'
    printf 'Something look off? Run mbx_doctor. Change settings: mbx_configure.\n'
    mkdir -p -- "${path%/*}" 2>/dev/null && : > "$path" 2>/dev/null || true
}

_mbx_engine_start || true
_mbx_install_hooks
_mbx_editor_install || true
_mbx_completion_install || true
_mbx_highlight_install || true
_mbx_search_install || true
_mbx_history_install_hooks
_mbx_ghost_install || true
_MBX_INITIALIZED=1
_mbx_first_run_notice

unset _MBX_INIT_FILE _MBX_BASH_DIR
