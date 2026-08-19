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
source "$_MBX_ROOT/bash/fallback.bash" || return 0
source "$_MBX_ROOT/bash/engine.bash" || return 0
source "$_MBX_ROOT/bash/prompt.bash" || return 0
source "$_MBX_ROOT/bash/hooks.bash" || return 0
source "$_MBX_ROOT/bash/editor.bash" || return 0
source "$_MBX_ROOT/bash/completion.bash" || return 0
source "$_MBX_ROOT/bash/history.bash" || return 0
source "$_MBX_ROOT/bash/ghost.bash" || return 0

if [[ -z ${MBX_BIN:-} ]]; then
    if [[ -x $_MBX_ROOT/target/release/mbx ]]; then
        MBX_BIN=$_MBX_ROOT/target/release/mbx
    else
        MBX_BIN=$_MBX_ROOT/target/debug/mbx
    fi
fi

_mbx_engine_start || true
_mbx_install_hooks
_mbx_editor_install || true
_mbx_completion_install || true
_mbx_history_install_hooks
_mbx_ghost_install || true
_MBX_INITIALIZED=1

unset _MBX_INIT_FILE _MBX_BASH_DIR
