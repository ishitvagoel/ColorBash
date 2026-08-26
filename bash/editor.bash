# shellcheck shell=bash
# Non-destructive Readline insertion via bind -x (ADR 0003). Inserts ordinary
# Bash text at READLINE_POINT without executing it.

_MBX_EDITOR_DEFAULT_KEYSEQ='\C-x\C-y'
_MBX_EDITOR_DEFAULT_TOKEN=$'printf \'MBX_EDT:ok\\n\''

_mbx_editor_insert_token() {
    local token=${MBX_EDITOR_INSERT_TOKEN:-$_MBX_EDITOR_DEFAULT_TOKEN}
    local point=${READLINE_POINT:-0}
    local line=${READLINE_LINE-}
    _mbx_text_has_c0_or_del "$token" && return 0
    READLINE_LINE=${line:0:point}${token}${line:point}
    READLINE_POINT=$((point + ${#token}))
}

_mbx_editor_keyseq_occupied() {
    local keyseq=$1
    local keymap=$2
    bind -m "$keymap" -X 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    bind -m "$keymap" -p 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    return 1
}

_mbx_editor_install_keymap() {
    local keymap=$1
    local keyseq=$2
    if _mbx_editor_keyseq_occupied "$keyseq" "$keymap" && [[ ${MBX_EDITOR_OVERRIDE:-0} != 1 ]]; then
        return 1
    fi
    bind -m "$keymap" -x "\"$keyseq\": _mbx_editor_insert_token"
}

_mbx_editor_install() {
    [[ ${_MBX_EDITOR_INSTALLED:-0} != 1 ]] || return 0
    local keyseq=${MBX_EDITOR_INSERT_KEYSEQ:-$_MBX_EDITOR_DEFAULT_KEYSEQ}
    _MBX_EDITOR_INSERT_BOUND=0
    _MBX_EDITOR_VI_INSERT_BOUND=0
    _MBX_EDITOR_INSERT_KEYSEQ_ACTIVE=$keyseq
    if _mbx_editor_install_keymap emacs "$keyseq"; then
        _MBX_EDITOR_INSERT_BOUND=1
    fi
    if _mbx_editor_install_keymap vi-insert "$keyseq"; then
        _MBX_EDITOR_VI_INSERT_BOUND=1
    fi
    _MBX_EDITOR_INSTALLED=1
}
