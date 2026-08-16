# shellcheck shell=bash
# Non-destructive Readline insertion via bind -x (ADR 0003). Inserts ordinary
# Bash text at READLINE_POINT without executing it.

_MBX_EDITOR_DEFAULT_KEYSEQ='\C-x\C-y'
_MBX_EDITOR_DEFAULT_TOKEN=$'printf \'MBX_EDT:ok\\n\''

_mbx_editor_insert_token() {
    local token=${MBX_EDITOR_INSERT_TOKEN:-$_MBX_EDITOR_DEFAULT_TOKEN}
    local point=${READLINE_POINT:-0}
    local line=${READLINE_LINE-}
    READLINE_LINE=${line:0:point}${token}${line:point}
    READLINE_POINT=$((point + ${#token}))
}

_mbx_editor_keyseq_occupied() {
    local keyseq=$1
    bind -X 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    bind -p 2>/dev/null | grep -Fq "\"$keyseq\":" && return 0
    return 1
}

_mbx_editor_install() {
    [[ ${_MBX_EDITOR_INSTALLED:-0} != 1 ]] || return 0
    local keyseq=${MBX_EDITOR_INSERT_KEYSEQ:-$_MBX_EDITOR_DEFAULT_KEYSEQ}
    _MBX_EDITOR_INSERT_BOUND=0
    _MBX_EDITOR_INSERT_KEYSEQ_ACTIVE=$keyseq
    if _mbx_editor_keyseq_occupied "$keyseq" && [[ ${MBX_EDITOR_OVERRIDE:-0} != 1 ]]; then
        _MBX_EDITOR_INSTALLED=1
        return 0
    fi
    bind -x "\"$keyseq\": _mbx_editor_insert_token"
    _MBX_EDITOR_INSERT_BOUND=1
    _MBX_EDITOR_INSTALLED=1
}
