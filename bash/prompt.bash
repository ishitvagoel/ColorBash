# shellcheck shell=bash
# Prompt fallback orchestration. Transport adapters and the fallback renderer
# all return through REPLY; this coordinator is the only PS1 writer.

_mbx_update_prompt() {
    local status=$1
    local duration_ms=${2:--}
    local cwd=$PWD
    local flags prompt
    local rendered=0
    local deadline_available=0
    local _MBX_RENDER_DEADLINE_US=

    if _mbx_render_deadline_start; then
        _MBX_RENDER_DEADLINE_US=$REPLY
        deadline_available=1
    fi
    _mbx_reap_children

    _mbx_prompt_flags
    flags=$REPLY

    if ((deadline_available == 1)) && [[ ${_MBX_ENGINE_READY:-0} == 1 ]]; then
        if _mbx_coprocess_requested && \
            _mbx_prompt_from_coprocess "$status" "$duration_ms" "$cwd" "$flags"; then
            prompt=$REPLY
            rendered=1
        else
            _mbx_engine_stop
        fi
    fi

    if ((rendered == 0 && deadline_available == 1)) && _mbx_per_call_available; then
        if _mbx_prompt_per_call "$status" "$duration_ms" "$cwd" "$flags"; then
            prompt=$REPLY
            rendered=1
        fi
    fi

    if ((rendered == 0)); then
        _mbx_fallback_prompt "$status" "$duration_ms" "$cwd" "$flags"
        prompt=$REPLY
    fi
    PS1=$prompt
}
