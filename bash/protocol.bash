# shellcheck shell=bash
# MBX1 constants, field encoding, and response validation.

_MBX_PROTOCOL_MAGIC=MBX1
_MBX_PROTOCOL_MAGIC_HISTORY=MBX2
_MBX_PROTOCOL_MAX_MESSAGE_BYTES=$((64 * 1024))
_MBX_PROTOCOL_FORBIDDEN_RAW_BYTES=$'\001\002\003\004\005\006\007\010\012\013\014\015\016\017\020\021\022\023\024\025\026\027\030\031\032\033\034\035\036\037\177'

_MBX_FLAG_NO_COLOR=$((1 << 0))
_MBX_FLAG_ASCII_ICONS=$((1 << 1))
_MBX_FLAG_NERD_ICONS=$((1 << 2))
_MBX_FLAG_SSH=$((1 << 3))
_MBX_FLAG_PRODUCTION=$((1 << 4))
_MBX_FLAG_DISABLE_GIT=$((1 << 5))
_MBX_FLAG_COLOR_16=$((1 << 6))
_MBX_FLAG_TRUECOLOR=$((1 << 7))

_mbx_escape_field() {
    (($# == 1 || $# == 2 || $# == 4)) || return 2

    local value=${1-}
    local max_bytes=${2-}
    local budget_check=${3-}
    local budget=${4-}
    local escaped= byte encoded
    local code index encoded_length=0 width
    local LC_ALL=C
    REPLY=

    if [[ -n $max_bytes ]]; then
        [[ $max_bytes =~ ^[0-9]+$ ]] || return 2
        # Every source byte needs at least one output byte. Reject impossible
        # values before entering the per-byte encoder or allocating a result.
        ((${#value} <= max_bytes)) || return 1
    fi
    if [[ -n $budget_check ]] && ! "$budget_check" "$budget" >/dev/null; then
        return 1
    fi
    # The overwhelmingly common path needs no escaping. Let Bash's native glob
    # matcher validate printable ASCII in one pass and avoid the costly byte
    # loop entirely. Percent is printable but reserved by MBX1.
    if [[ $value != *%* && $value != *[!\ -~]* ]]; then
        REPLY=$value
        return 0
    fi

    # Encoding non-ASCII bytes as well as control bytes keeps the Bash encoder
    # independent of locale-specific character classes. The Rust decoder
    # reconstructs the original UTF-8 bytes.
    for ((index = 0; index < ${#value}; index++)); do
        if [[ -n $budget_check ]] && ((index > 0 && index % 32 == 0)) && \
            ! "$budget_check" "$budget" >/dev/null; then
            REPLY=
            return 1
        fi
        byte=${value:index:1}
        printf -v code '%d' "'$byte"
        if ((code >= 32 && code <= 126 && code != 37)); then
            encoded=$byte
            width=1
        else
            printf -v encoded '%%%02X' "$code"
            width=3
        fi
        ((encoded_length += width))
        if [[ -n $max_bytes ]] && ((encoded_length > max_bytes)); then
            REPLY=
            return 1
        fi
        escaped+=$encoded
    done
    if [[ -n $budget_check ]] && ! "$budget_check" "$budget" >/dev/null; then
        REPLY=
        return 1
    fi
    REPLY=$escaped
}

_mbx_unescape_field() {
    (($# == 1 || $# == 3)) || return 2

    local remaining=${1-}
    local budget_check=${2-}
    local budget=${3-}
    local decoded= prefix hex byte
    local escape_pattern='^([^%]*)%(.{2})(.*)$'
    local code
    local LC_ALL=C
    REPLY=

    while [[ $remaining == *%* ]]; do
        if [[ -n $budget_check ]] && ! "$budget_check" "$budget" >/dev/null; then
            REPLY=
            return 1
        fi
        [[ $remaining =~ $escape_pattern ]] || return 1
        prefix=${BASH_REMATCH[1]}
        hex=${BASH_REMATCH[2]}
        remaining=${BASH_REMATCH[3]}
        decoded+=$prefix
        [[ $hex =~ ^[0-9A-Fa-f]{2}$ ]] || return 1
        code=$((16#$hex))
        # Bash variables cannot represent NUL, and MBX1 forbids it.
        ((code != 0)) || return 1
        printf -v byte '%b' "\\x$hex"
        decoded+=$byte
    done

    if [[ -n $budget_check ]] && ! "$budget_check" "$budget" >/dev/null; then
        REPLY=
        return 1
    fi
    REPLY=$decoded$remaining
}

_mbx_protocol_validate_line() {
    local line=${1-}
    local LC_ALL=C

    ((${#line} <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || return 1
    # Tabs are MBX1 field separators. A single Bash pattern performs the raw
    # control scan in native code instead of spending seconds in a byte loop at
    # the 64-KiB boundary.
    [[ $line != *["$_MBX_PROTOCOL_FORBIDDEN_RAW_BYTES"]* ]]
}

_mbx_protocol_split_fields() {
    (($# == 2 || $# == 4)) || return 2

    local remaining=${1-}
    local output_name=$2
    local budget_check=${3-}
    local budget=${4-}
    local -n output=$output_name
    local separator_pattern=$'^([^\t]*)\t(.*)$'
    local field next
    local LC_ALL=C

    output=()
    while [[ $remaining =~ $separator_pattern ]]; do
        field=${BASH_REMATCH[1]}
        next=${BASH_REMATCH[2]}
        if [[ -n $budget_check ]] && ! "$budget_check" "$budget" >/dev/null; then
            return 1
        fi
        output+=("$field")
        remaining=$next
    done
    output+=("$remaining")
}

_mbx_protocol_encode_ping() {
    local request_id=$1
    printf -v REPLY '%s\t%s\tPING' "$_MBX_PROTOCOL_MAGIC" "$request_id"
}

_mbx_protocol_encode_prompt() {
    (($# == 5 || $# == 7)) || return 2

    local request_id=$1
    local cwd=$2
    local status=$3
    local duration_ms=$4
    local flags=$5
    local budget_check=${6-}
    local budget=${7-}
    local escaped_cwd prefix suffix
    local available_bytes
    local LC_ALL=C
    REPLY=

    printf -v prefix '%s\t%s\tPROMPT\t' "$_MBX_PROTOCOL_MAGIC" "$request_id"
    printf -v suffix '\t%s\t%s\t%s' "$status" "$duration_ms" "$flags"
    available_bytes=$((_MBX_PROTOCOL_MAX_MESSAGE_BYTES - ${#prefix} - ${#suffix}))
    ((available_bytes >= 0)) || return 1

    if [[ -n $budget_check ]]; then
        _mbx_escape_field "$cwd" "$available_bytes" "$budget_check" "$budget" || return 1
    else
        _mbx_escape_field "$cwd" "$available_bytes" || return 1
    fi
    escaped_cwd=$REPLY
    REPLY=$prefix$escaped_cwd$suffix
    ((${#REPLY} <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || {
        REPLY=
        return 1
    }
}

_mbx_protocol_decode_pong() {
    (($# == 2 || $# == 4)) || return 2

    local expected_id=$1
    local line=$2
    local budget_check=${3-}
    local budget=${4-}
    local -a fields=()

    REPLY=
    _mbx_protocol_validate_line "$line" || return 1
    if [[ -n $budget_check ]]; then
        _mbx_protocol_split_fields "$line" fields "$budget_check" "$budget" || return 1
    else
        _mbx_protocol_split_fields "$line" fields
    fi
    ((${#fields[@]} == 3)) || return 1
    [[ ${fields[0]} == "$_MBX_PROTOCOL_MAGIC" && \
        ${fields[1]} == "$expected_id" && \
        ${fields[2]} == PONG ]] || return 1
    [[ -z $budget_check ]] || "$budget_check" "$budget" >/dev/null
}

_mbx_protocol_decode_prompt() {
    (($# == 2 || $# == 4)) || return 2

    local expected_id=$1
    local line=$2
    local budget_check=${3-}
    local budget=${4-}
    local -a fields=()

    REPLY=
    _mbx_protocol_validate_line "$line" || return 1
    if [[ -n $budget_check ]]; then
        _mbx_protocol_split_fields "$line" fields "$budget_check" "$budget" || return 1
    else
        _mbx_protocol_split_fields "$line" fields
    fi
    ((${#fields[@]} == 4)) || return 1
    [[ ${fields[0]} == "$_MBX_PROTOCOL_MAGIC" && \
        ${fields[1]} == "$expected_id" && \
        ${fields[2]} == PROMPT ]] || return 1
    if [[ -n $budget_check ]]; then
        _mbx_unescape_field "${fields[3]}" "$budget_check" "$budget"
    else
        _mbx_unescape_field "${fields[3]}"
    fi
}

_mbx_protocol_encode_history_record() {
    (($# == 11)) || return 2

    local request_id=$1
    local -a raw=( "${@:2}" )
    local payload= field
    local index

    # Every record field travels percent-escaped so hostile command text and
    # paths cannot break framing; "-" sentinels for null numbers stay literal.
    for ((index = 0; index < ${#raw[@]}; index++)); do
        if ((index == 2 || index == 7)); then
            field=${raw[index]}
            if [[ $field == '-' ]]; then
                payload+=$field
            else
                _mbx_escape_field "$field" || return 1
                payload+=$REPLY
            fi
        else
            _mbx_escape_field "${raw[index]}" || return 1
            payload+=$REPLY
        fi
        ((index + 1 < ${#raw[@]})) && payload+=$'\t'
    done
    printf -v REPLY '%s\t%s\tRECORD\t%s' \
        "$_MBX_PROTOCOL_MAGIC_HISTORY" "$request_id" "$payload"
    ((${#REPLY} <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || {
        REPLY=
        return 1
    }
}

_mbx_protocol_decode_history_ack() {
    (($# == 2)) || return 2

    local expected_id=$1
    local line=$2
    local -a fields=()

    REPLY=
    _mbx_protocol_validate_line "$line" || return 1
    _mbx_protocol_split_fields "$line" fields
    ((${#fields[@]} == 3)) || return 1
    [[ ${fields[0]} == "$_MBX_PROTOCOL_MAGIC_HISTORY" && \
        ${fields[1]} == "$expected_id" && \
        ${fields[2]} == ACK ]] || return 1
}

# QUERY: request_id generation mode text-or-dash limit → REPLY wire line.
_mbx_protocol_encode_history_query() {
    (($# == 5)) || return 2

    local request_id=$1
    local generation=$2
    local mode=$3
    local text=$4
    local limit=$5
    local escaped_generation escaped_mode escaped_text escaped_limit

    REPLY=
    _mbx_escape_field "$generation" || return 1
    escaped_generation=$REPLY
    _mbx_escape_field "$mode" || return 1
    escaped_mode=$REPLY
    if [[ $text == '-' ]]; then
        escaped_text='-'
    else
        _mbx_escape_field "$text" || return 1
        escaped_text=$REPLY
    fi
    _mbx_escape_field "$limit" || return 1
    escaped_limit=$REPLY
    printf -v REPLY '%s\t%s\tQUERY\t%s\t%s\t%s\t%s' \
        "$_MBX_PROTOCOL_MAGIC_HISTORY" "$request_id" \
        "$escaped_generation" "$escaped_mode" "$escaped_text" "$escaped_limit"
    ((${#REPLY} <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || {
        REPLY=
        return 1
    }
}

# CANCEL: request_id generation → REPLY wire line.
_mbx_protocol_encode_history_cancel() {
    (($# == 2)) || return 2

    local request_id=$1
    local generation=$2
    local escaped_generation

    REPLY=
    _mbx_escape_field "$generation" || return 1
    escaped_generation=$REPLY
    printf -v REPLY '%s\t%s\tCANCEL\t%s' \
        "$_MBX_PROTOCOL_MAGIC_HISTORY" "$request_id" "$escaped_generation"
    ((${#REPLY} <= _MBX_PROTOCOL_MAX_MESSAGE_BYTES)) || {
        REPLY=
        return 1
    }
}

# RESULT frame: line dest_array_name → REPLY=generation; dest filled.
# Does not require a request-id match so a delayed stale RESULT can be skipped.
_mbx_protocol_parse_history_result() {
    (($# == 2)) || return 2

    local line=$1
    local -n _mbx_result_cmds=$2
    local -a fields=()
    local count index decoded

    REPLY=
    _mbx_result_cmds=()
    _mbx_protocol_validate_line "$line" || return 1
    _mbx_protocol_split_fields "$line" fields
    ((${#fields[@]} >= 5)) || return 1
    [[ ${fields[0]} == "$_MBX_PROTOCOL_MAGIC_HISTORY" && \
        ${fields[2]} == RESULT ]] || return 1
    _mbx_unescape_field "${fields[3]}" || return 1
    decoded=$REPLY
    [[ $decoded =~ ^[0-9]+$ ]] || return 1
    REPLY=$decoded
    _mbx_unescape_field "${fields[4]}" || return 1
    count=$REPLY
    [[ $count =~ ^[0-9]+$ ]] || return 1
    ((${#fields[@]} == 5 + count)) || return 1
    _mbx_result_cmds=()
    for ((index = 0; index < count; index++)); do
        _mbx_unescape_field "${fields[5 + index]}" || return 1
        _mbx_result_cmds+=("$REPLY")
    done
    REPLY=$decoded
}

# RESULT: expected_id line dest_array_name → REPLY=generation; dest filled.
_mbx_protocol_decode_history_result() {
    (($# == 3)) || return 2

    local expected_id=$1
    local line=$2
    local dest=$3
    local -a fields=()

    REPLY=
    _mbx_protocol_validate_line "$line" || return 1
    _mbx_protocol_split_fields "$line" fields
    ((${#fields[@]} >= 5)) || return 1
    [[ ${fields[1]} == "$expected_id" ]] || return 1
    _mbx_protocol_parse_history_result "$line" "$dest"
}

# ERROR: expected_id line → REPLY=kind
_mbx_protocol_decode_history_error() {
    (($# == 2)) || return 2

    local expected_id=$1
    local line=$2
    local -a fields=()

    REPLY=
    _mbx_protocol_validate_line "$line" || return 1
    _mbx_protocol_split_fields "$line" fields
    ((${#fields[@]} == 4)) || return 1
    [[ ${fields[0]} == "$_MBX_PROTOCOL_MAGIC_HISTORY" && \
        ${fields[1]} == "$expected_id" && \
        ${fields[2]} == ERROR ]] || return 1
    _mbx_unescape_field "${fields[3]}"
}
