#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/../.." && pwd -P)
MBX_TEST_BIN=${1:-"$ROOT/target/debug/mbx"}

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

assert_eq() {
    local expected=$1
    local actual=$2
    local message=$3
    if [[ $actual != "$expected" ]]; then
        printf 'FAIL: %s\n  expected: %q\n    actual: %q\n' \
            "$message" "$expected" "$actual" >&2
        exit 1
    fi
}

assert_status() {
    local expected=$1
    local message=$2
    shift 2
    local actual=0

    "$@" || actual=$?
    assert_eq "$expected" "$actual" "$message"
}

source "$ROOT/bash/protocol.bash"
source "$ROOT/bash/config.bash"
source "$ROOT/bash/fallback.bash"
source "$ROOT/bash/engine.bash"
source "$ROOT/bash/prompt.bash"

hostile_field=$'a%\t\n\r\e\177\303\251'
_mbx_escape_field "$hostile_field"
assert_eq 'a%25%09%0A%0D%1B%7F%C3%A9' "$REPLY" \
    'the Bash protocol encoder did not escape every unsafe byte'
encoded_field=$REPLY
_mbx_unescape_field "$encoded_field"
assert_eq "$hostile_field" "$REPLY" 'the Bash field codec did not round-trip'

_mbx_unescape_field '%5c%4a'
assert_eq '\J' "$REPLY" 'lowercase percent escapes were not decoded'
for malformed_field in '%' '%0' '%0Z' '%GG' '%00'; do
    if _mbx_unescape_field "$malformed_field"; then
        fail "malformed protocol field was accepted: $malformed_field"
    fi
done

_mbx_protocol_decode_pong 7 $'MBX1\t7\tPONG' || fail 'valid PONG response was rejected'
if _mbx_protocol_decode_pong 7 $'MBX1\t7\tPONG\t'; then
    fail 'PONG response with an extra empty field was accepted'
fi
_mbx_protocol_decode_prompt 8 $'MBX1\t8\tPROMPT\t'
assert_eq '' "$REPLY" 'an empty prompt payload did not retain its field'
if _mbx_protocol_decode_prompt 8 $'MBX1\t8\tPROMPT\t\tshifted'; then
    fail 'consecutive separators bypassed prompt field-count validation'
fi
if _mbx_protocol_decode_prompt 8 $'MBX1\t8\tPROMPT\tbad%0Z'; then
    fail 'a malformed prompt payload was accepted'
fi
if _mbx_protocol_decode_prompt 8 $'MBX1\t8\tPROMPT\tbad\evalue'; then
    fail 'an unescaped response control character was accepted'
fi

unset NO_COLOR MBX_COLOR SSH_TTY
TERM=dumb
MBX_ICONS=nerd
SSH_CONNECTION='client server'
MBX_PRODUCTION_CONTEXT=1
MBX_DISABLE_GIT=1
_mbx_prompt_flags
expected_flags=$((_MBX_FLAG_NO_COLOR | _MBX_FLAG_NERD_ICONS | _MBX_FLAG_SSH | \
    _MBX_FLAG_PRODUCTION | _MBX_FLAG_DISABLE_GIT))
assert_eq "$expected_flags" "$REPLY" 'prompt environment policy produced the wrong flags'

assert_status 2 'the coprocess adapter accepted an incomplete context' \
    _mbx_prompt_from_coprocess 0 - /tmp
assert_status 2 'the per-call adapter accepted an incomplete context' \
    _mbx_prompt_per_call 0 - /tmp
assert_status 2 'the fallback adapter accepted an incomplete context' \
    _mbx_fallback_prompt 0 - /tmp

unknown_flag=$((1 << 20))
forward_flags=$((expected_flags | unknown_flag))
PS1=unchanged
MBX_BIN=/bin/echo
MBX_RENDER_TIMEOUT=.25
_mbx_prompt_per_call 7 2500 /tmp/project "$forward_flags"
assert_eq unchanged "$PS1" 'the per-call adapter mutated PS1'
assert_eq \
    "prompt --cwd /tmp/project --status 7 --flags $forward_flags --duration-ms 2500" \
    "$REPLY" 'the per-call adapter did not preserve the raw additive flags'

PS1=unchanged
_mbx_fallback_prompt 5 2500 /home/test/work \
    "$((_MBX_FLAG_NO_COLOR | _MBX_FLAG_DISABLE_GIT))"
assert_eq unchanged "$PS1" 'the fallback adapter mutated PS1'
assert_eq '/home/test/work  exit 5  2s\n> ' "$REPLY" \
    'the fallback adapter did not render the explicit context'

# Even with repository rendering enabled, the last-resort prompt must remain a
# builtin-only path and must not discover or invoke Git.
command() { fail 'the fallback invoked command'; }
git() { fail 'the fallback invoked git'; }
_mbx_fallback_prompt 0 - /tmp "$_MBX_FLAG_NO_COLOR"
assert_eq '/tmp\n> ' "$REPLY" 'the process-free fallback changed its base prompt'
unset -f command git

fallback_256_flags=$((_MBX_FLAG_ASCII_ICONS | _MBX_FLAG_DISABLE_GIT))
_mbx_fallback_prompt 0 - /tmp/project "$fallback_256_flags"
[[ $REPLY == *'38;5;'* ]] || fail '256-color fallback path must use 38;5 SGR'
[[ $REPLY != *'38;2;'* && $REPLY != *'1;36m'* ]] || \
    fail '256-color fallback path must not use truecolor or 16-color SGR'

fallback_truecolor_flags=$((fallback_256_flags | _MBX_FLAG_TRUECOLOR))
_mbx_fallback_prompt 0 - /tmp/project "$fallback_truecolor_flags"
[[ $REPLY == *'38;2;'* ]] || fail 'truecolor fallback path must use 38;2 SGR'
[[ $REPLY != *'38;5;'* ]] || fail 'truecolor fallback path must not use 256-color SGR'

fallback_16_flags=$((_MBX_FLAG_ASCII_ICONS | _MBX_FLAG_DISABLE_GIT | _MBX_FLAG_COLOR_16))
_mbx_fallback_prompt 0 - /tmp/project "$fallback_16_flags"
[[ $REPLY == *'1;36'* ]] || fail '16-color fallback path must use 1;36 SGR'
[[ $REPLY != *'38;5;'* && $REPLY != *'38;2;'* ]] || \
    fail '16-color fallback path must not use 256 or truecolor SGR'

HOSTNAME='prod$host'
USER='root\user'
_mbx_fallback_prompt 0 - /srv/app \
    "$((_MBX_FLAG_NO_COLOR | _MBX_FLAG_DISABLE_GIT | _MBX_FLAG_PRODUCTION | _MBX_FLAG_SSH))"
assert_eq '! PROD · prod?host · root?user  /srv/app\n> ' "$REPLY" \
    'production did not retain precedence over SSH in the fallback'

HOSTNAME=$'remote\001$host`name\\tail'
_mbx_fallback_prompt 0 - /srv/app \
    "$((_MBX_FLAG_NO_COLOR | _MBX_FLAG_DISABLE_GIT | _MBX_FLAG_SSH))"
assert_eq 'ssh: remote??host?name?tail  /srv/app\n> ' "$REPLY" \
    'the SSH-only fallback lost or failed to sanitize its context'

# Build one hostile corpus for every renderer: all representable C0 controls,
# DEL, and the three characters that Bash expands while decoding PS1.
hostile_controls=
for ((control = 1; control <= 31; control++)); do
    printf -v octal '%03o' "$control"
    printf -v byte '%b' "\\$octal"
    hostile_controls+=$byte
done
printf -v byte '%b' '\177'
hostile_controls+=$byte
hostile_controls+='$'
hostile_controls+='`'
hostile_controls+='\'
printf -v hostile_replacement '%*s' 35 ''
hostile_replacement=${hostile_replacement// /?}
hostile_cwd="/tmp/$hostile_controls"
safe_cwd="/tmp/$hostile_replacement"
native_flags=$((_MBX_FLAG_NO_COLOR | _MBX_FLAG_ASCII_ICONS | \
    _MBX_FLAG_DISABLE_GIT | unknown_flag))
expected_hostile_prompt="${safe_cwd}\\n> "

for ((control = 1; control <= 31; control++)); do
    ((control == 9)) && continue
    printf -v octal '%03o' "$control"
    printf -v byte '%b' "\\$octal"
    if _mbx_protocol_validate_line "MBX1${byte}unsafe"; then
        fail "the optimized protocol scan accepted raw control byte $control"
    fi
done
if _mbx_protocol_validate_line $'MBX1\177unsafe'; then
    fail 'the optimized protocol scan accepted raw DEL'
fi

_mbx_sanitize_text "$hostile_controls"
assert_eq "$hostile_replacement" "$REPLY" \
    'the fallback sanitizer did not replace the complete hostile corpus'
_mbx_fallback_prompt 0 - "$hostile_cwd" "$native_flags"
assert_eq "$expected_hostile_prompt" "$REPLY" \
    'the fallback renderer violated the shared hostile-state contract'

[[ -x $MBX_TEST_BIN ]] || fail "mbx binary is missing: $MBX_TEST_BIN"
MBX_BIN=$MBX_TEST_BIN
MBX_RENDER_TIMEOUT=.25
_mbx_prompt_per_call 0 - "$hostile_cwd" "$native_flags" || \
    fail 'the per-call adapter rejected the hostile corpus'
assert_eq "$expected_hostile_prompt" "$REPLY" \
    'the per-call renderer violated the shared hostile-state contract'

# Run the real CLI below command substitution with color explicitly enabled.
# Its stdout is therefore a pipe, so only the forwarded capability flags can
# preserve color; ambient isatty inference cannot make this pass.
color_flags=$((_MBX_FLAG_ASCII_ICONS | _MBX_FLAG_DISABLE_GIT | unknown_flag))
colored_prompt=$(
    _mbx_prompt_per_call 0 - /tmp/color "$color_flags" || exit 1
    printf '%s' "$REPLY"
)
[[ $colored_prompt == *'\[\e['* ]] || \
    fail 'real command substitution stripped color from the per-call adapter'

MBX_IPC_MODE=coprocess
MBX_DISABLE_RENDERER=0
MBX_IPC_TIMEOUT=.25
_mbx_engine_start || fail 'the coprocess engine did not become ready'
PS1=unchanged
_mbx_prompt_from_coprocess 7 2500 /tmp/project "$native_flags" || \
    fail 'the coprocess prompt adapter failed'
assert_eq unchanged "$PS1" 'the coprocess adapter mutated PS1'
assert_eq '/tmp/project  exit 7  2.5s\n> ' "$REPLY" \
    'the coprocess adapter returned an unexpected prompt'
_mbx_prompt_from_coprocess 0 - "$hostile_cwd" "$native_flags" || \
    fail 'the coprocess adapter rejected the hostile corpus'
assert_eq "$expected_hostile_prompt" "$REPLY" \
    'the coprocess renderer violated the shared hostile-state contract'
_mbx_engine_stop

wait_for_deferred_reap() {
    local deadline

    _mbx_deadline_after .50
    deadline=$REPLY
    while [[ -n ${_MBX_DEFERRED_CHILD_PIDS:-} ]]; do
        _mbx_reap_children
        [[ -z ${_MBX_DEFERRED_CHILD_PIDS:-} ]] && return 0
        _mbx_deadline_remaining "$deadline" >/dev/null || \
            fail 'a terminated Bash-owned helper was not reaped'
    done
}
wait_for_deferred_reap

MBX_BIN=/bin/true
MBX_IPC_MODE=auto
if _mbx_engine_start; then
    fail 'engine startup reported success without a successful handshake'
fi
assert_eq 0 "${_MBX_ENGINE_READY:-missing}" 'failed engine startup left an invalid ready state'
[[ -z ${_MBX_ENGINE_CHILD_PID:-}${_MBX_ENGINE_IN_FD:-}${_MBX_ENGINE_OUT_FD:-} ]] || \
    fail 'failed engine startup retained process resources'
_mbx_engine_stop
_mbx_engine_stop
wait_for_deferred_reap

run_bounded_read_case() {
    local size=$1
    local terminator=$2
    local should_accept=$3
    local label=$4
    local payload fd producer deadline result=
    local actual_status=0

    printf -v payload '%*s' "$size" ''
    exec {fd}< <(printf '%s%s' "$payload" "$terminator")
    producer=$!
    _mbx_deadline_after .50
    deadline=$REPLY
    if _mbx_read_bounded_response "$fd" "$deadline"; then
        result=$REPLY
    else
        actual_status=$?
    fi
    exec {fd}<&-
    if kill -0 "$producer" 2>/dev/null; then
        kill -KILL "$producer" 2>/dev/null || true
    fi
    wait "$producer" 2>/dev/null || true

    if ((should_accept == 1)); then
        ((actual_status == 0)) || fail "$label was rejected"
        [[ $result == "$payload" ]] || fail "$label returned a truncated payload"
    elif ((actual_status == 0)); then
        fail "$label bypassed the MAX+1 acquisition guard"
    fi
}

for terminator_name in EOF LF CRLF; do
    case $terminator_name in
        EOF) terminator= ;;
        LF) terminator=$'\n' ;;
        CRLF) terminator=$'\r\n' ;;
    esac
    run_bounded_read_case "$((_MBX_PROTOCOL_MAX_MESSAGE_BYTES - 1))" \
        "$terminator" 1 "MAX-1/$terminator_name"
    run_bounded_read_case "$_MBX_PROTOCOL_MAX_MESSAGE_BYTES" \
        "$terminator" 1 "MAX/$terminator_name"
    run_bounded_read_case "$((_MBX_PROTOCOL_MAX_MESSAGE_BYTES + 1))" \
        "$terminator" 0 "MAX+1/$terminator_name"
done

emit_multi_mib_response() {
    local terminator=$1
    local completion_marker=$2
    local block

    # Produce 2 MiB without first constructing it in the test process. If the
    # bounded reader stops at MAX+1, closing its fd interrupts this producer
    # before it can finish and create the marker.
    for ((block = 0; block < 32; block++)); do
        printf '%65536s' '' || return 1
    done
    [[ $terminator == LF ]] && printf '\n'
    : >"$completion_marker"
}

run_multi_mib_rejection() {
    local terminator=$1
    local label=$2
    local completion_marker=${TMPDIR:-/tmp}/colorbash-oversize-$BASHPID-$RANDOM
    local fd producer deadline started_us elapsed_us

    rm -f -- "$completion_marker"
    exec {fd}< <(emit_multi_mib_response "$terminator" "$completion_marker" 2>/dev/null)
    producer=$!
    _mbx_deadline_after .25
    deadline=$REPLY
    _mbx_clock_now_us
    started_us=$REPLY
    if _mbx_read_bounded_response "$fd" "$deadline"; then
        fail "$label multi-MiB response bypassed the MAX+1 acquisition guard"
    fi
    _mbx_clock_now_us
    elapsed_us=$((REPLY - started_us))
    exec {fd}<&-
    if kill -0 "$producer" 2>/dev/null; then
        kill -KILL "$producer" 2>/dev/null || true
    fi
    wait "$producer" 2>/dev/null || true

    ((elapsed_us < 400000)) || fail "$label multi-MiB rejection was not prompt"
    [[ ! -e $completion_marker ]] || \
        fail "$label multi-MiB producer was collected in full before rejection"
    rm -f -- "$completion_marker"
}

run_multi_mib_rejection EOF 'unterminated'
run_multi_mib_rejection LF 'LF-terminated'

# Bash normally drops NUL while reading. The empty delimiter makes it observable
# and rejects even a valid frame hidden behind a leading NUL.
exec {nul_fd}< <(printf '\0MBX1\t1\tPONG\n')
nul_pid=$!
_mbx_deadline_after .25
if _mbx_read_bounded_response "$nul_fd" "$REPLY"; then
    fail 'a NUL-prefixed valid frame bypassed the acquisition guard'
fi
exec {nul_fd}<&-
wait "$nul_pid" 2>/dev/null || true

exec {nul_fd}< <(printf 'M\0BX1\t1\tPONG\n')
nul_pid=$!
_mbx_deadline_after .25
if _mbx_read_bounded_response "$nul_fd" "$REPLY"; then
    fail 'an embedded NUL bypassed the bulk acquisition guard'
fi
exec {nul_fd}<&-
wait "$nul_pid" 2>/dev/null || true

# Exercise the exact-MAX CRLF lookahead's negative branch with an actual pending
# byte. A non-LF byte must never be mistaken for the remainder of CRLF.
printf -v max_payload '%*s' "$_MBX_PROTOCOL_MAX_MESSAGE_BYTES" ''
exec {bad_lookahead_fd}< <(printf '%s\rZ' "$max_payload")
bad_lookahead_pid=$!
_mbx_deadline_after .25
if _mbx_read_bounded_response "$bad_lookahead_fd" "$REPLY"; then
    fail 'the exact-MAX CRLF lookahead accepted a non-LF byte'
fi
exec {bad_lookahead_fd}<&-
wait "$bad_lookahead_pid" 2>/dev/null || true

exec {stalled_lookahead_fd}< <(printf '%s\r' "$max_payload"; exec sleep 60)
stalled_lookahead_pid=$!
_mbx_deadline_after .03
lookahead_deadline=$REPLY
_mbx_clock_now_us
lookahead_started_us=$REPLY
if _mbx_read_bounded_response "$stalled_lookahead_fd" "$lookahead_deadline"; then
    fail 'the exact-MAX CRLF lookahead accepted a missing LF'
fi
_mbx_clock_now_us
lookahead_elapsed_us=$((REPLY - lookahead_started_us))
exec {stalled_lookahead_fd}<&-
kill -KILL "$stalled_lookahead_pid" 2>/dev/null || true
wait "$stalled_lookahead_pid" 2>/dev/null || true
((lookahead_elapsed_us < 200000)) || \
    fail 'the exact-MAX CRLF lookahead exceeded its absolute deadline'

stalling_bin="$ROOT/tests/bash/fixtures/stalling-mbx.bash"
marker=${TMPDIR:-/tmp}/colorbash-stall-$BASHPID-$RANDOM
rm -f -- "$marker"
export MBX_STALL_PROMPT_MARKER=$marker

# Request framing rejects a raw cwd that cannot possibly fit before entering the
# per-byte escape loop or allocating a protocol request.
printf -v oversized_logical_pwd '%*s' "$_MBX_PROTOCOL_MAX_MESSAGE_BYTES" ''
_mbx_clock_now_us
started_us=$REPLY
if _mbx_protocol_encode_prompt 1 "$oversized_logical_pwd" 0 - 0; then
    fail 'the outbound encoder accepted a request larger than the MBX1 maximum'
fi
failed_request=$REPLY
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 100000)) || fail 'oversized request preflight entered the slow encoder'
assert_eq '' "$failed_request" 'oversized request preflight retained a partial frame'

# The five-argument codec API remains valid. A printable-ASCII cwd that exactly
# fills the remaining frame capacity must take the native fast path and produce
# a request at, but never beyond, MAX.
printf -v request_prefix '%s\t%s\tPROMPT\t' "$_MBX_PROTOCOL_MAGIC" 1
printf -v request_suffix '\t%s\t%s\t%s' 0 - 0
near_limit_size=$((_MBX_PROTOCOL_MAX_MESSAGE_BYTES - \
    ${#request_prefix} - ${#request_suffix}))
printf -v exactly_fitting_cwd '%*s' "$near_limit_size" ''
_mbx_clock_now_us
started_us=$REPLY
_mbx_protocol_encode_prompt 1 "$exactly_fitting_cwd" 0 - 0 || \
    fail 'an exactly fitting printable-ASCII request was rejected'
encoded_request=$REPLY
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
assert_eq "$_MBX_PROTOCOL_MAX_MESSAGE_BYTES" "${#encoded_request}" \
    'the exactly fitting request did not land on the MBX1 boundary'
((elapsed_us < 100000)) || fail 'printable-ASCII request bypassed the native fast path'
unset encoded_request

# Percent must use the escape loop. Its cooperative deadline prevents an
# escape-heavy value from spending seconds constructing a doomed request.
printf -v escape_heavy_logical_pwd '%%%.0s' {1..22000}
_mbx_deadline_after .03
escape_deadline=$REPLY
_mbx_clock_now_us
started_us=$REPLY
if _mbx_protocol_encode_prompt \
    1 "$escape_heavy_logical_pwd" 0 - 0 _mbx_deadline_remaining "$escape_deadline"; then
    fail 'an oversized escape-heavy request was accepted'
fi
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 200000)) || fail 'escape-heavy request encoding escaped its deadline'

# A healthy handshake followed by an oversized logical PWD must still reach the
# builtin fallback within one render budget. Per-call may use only what remains;
# it cannot receive a fresh timeout after coprocess framing fails.
MBX_BIN=$stalling_bin
MBX_IPC_MODE=coprocess
MBX_IPC_TIMEOUT=.25
MBX_RENDER_TIMEOUT=.03
_mbx_engine_start || fail 'the oversized-PWD fixture did not complete its handshake'
_mbx_clock_now_us
started_us=$REPLY
PWD=$oversized_logical_pwd _mbx_update_prompt 0 -
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 200000)) || fail 'oversized request fallback escaped the render deadline'
assert_eq 0 "${_MBX_ENGINE_READY:-missing}" \
    'oversized request framing left the coprocess marked ready'
(( ${#PS1} < 1024 )) || fail 'the oversized logical PWD replaced the bounded fallback'
wait_for_deferred_reap
rm -f -- "$marker"

# A safe near-limit cwd must be encoded and sent through the healthy coprocess.
# Its stalled response consumes the existing budget and cannot grant per-call a
# fresh timeout.
printf -v request_prefix '%s\t%s\tPROMPT\t' "$_MBX_PROTOCOL_MAGIC" 2
printf -v request_suffix '\t%s\t%s\t%s' 0 - "$expected_flags"
near_limit_size=$((_MBX_PROTOCOL_MAX_MESSAGE_BYTES - \
    ${#request_prefix} - ${#request_suffix}))
printf -v near_limit_logical_pwd '%*s' "$near_limit_size" ''
serve_prompt_marker=${TMPDIR:-/tmp}/colorbash-serve-prompt-$BASHPID-$RANDOM
rm -f -- "$marker" "$serve_prompt_marker"
export MBX_STALL_SERVE_PROMPT_MARKER=$serve_prompt_marker
MBX_RENDER_TIMEOUT=.10
_mbx_engine_start || fail 'the near-limit-PWD fixture did not complete its handshake'
_mbx_clock_now_us
started_us=$REPLY
PWD=$near_limit_logical_pwd _mbx_update_prompt 0 -
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 200000)) || fail 'near-limit request encoding escaped the render deadline'
[[ -e $serve_prompt_marker ]] || fail 'the fitting near-limit request was not sent'
[[ ! -e $marker ]] || fail 'near-limit rendering granted per-call a second budget'
assert_eq 0 "${_MBX_ENGINE_READY:-missing}" \
    'expired near-limit request encoding left the coprocess marked ready'
(( ${#PS1} < 1024 )) || fail 'the near-limit logical PWD replaced the bounded fallback'
wait_for_deferred_reap
unset MBX_STALL_SERVE_PROMPT_MARKER
rm -f -- "$serve_prompt_marker"
unset oversized_logical_pwd exactly_fitting_cwd near_limit_logical_pwd \
    escape_heavy_logical_pwd

# A direct per-call timeout exercises process-substitution $! ownership and
# proves that the child is killed and later reaped without an unbounded wait.
MBX_BIN=$stalling_bin
MBX_RENDER_TIMEOUT=.03
_mbx_clock_now_us
started_us=$REPLY
if _mbx_prompt_per_call 0 - /tmp "$native_flags"; then
    fail 'a stalling per-call helper produced a prompt'
fi
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 200000)) || fail 'the per-call helper exceeded its absolute deadline'
[[ -e $marker ]] || fail 'the process-substitution helper was not actually started'
[[ -n ${_MBX_DEFERRED_CHILD_PIDS:-} ]] || \
    fail 'the timed-out process-substitution child was not retained for safe reaping'
wait_for_deferred_reap
rm -f -- "$marker"

# A syntactically valid near-MAX response with thousands of percent escapes is
# fully acquired before decoding. Decoding must still share the render deadline
# rather than extending prompt latency by seconds.
response_marker=${TMPDIR:-/tmp}/colorbash-response-$BASHPID-$RANDOM
rm -f -- "$response_marker" "$marker"
export MBX_STALL_RESPONSE_MODE=percent-heavy
export MBX_STALL_RESPONSE_MARKER=$response_marker
MBX_BIN=$stalling_bin
MBX_IPC_MODE=coprocess
MBX_IPC_TIMEOUT=.50
MBX_RENDER_TIMEOUT=.20
_mbx_engine_start || fail 'the percent-heavy fixture did not complete its handshake'
_mbx_clock_now_us
started_us=$REPLY
_mbx_update_prompt 0 -
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 400000)) || fail 'percent-heavy decoding escaped the render deadline'
[[ -e $response_marker ]] || fail 'the near-MAX response was not fully emitted before fallback'
[[ ! -e $marker ]] || fail 'percent-heavy decoding granted per-call a second budget'
(( ${#PS1} < 1024 )) || fail 'the expired percent-heavy response replaced the fallback prompt'
assert_eq 0 "${_MBX_ENGINE_READY:-missing}" \
    'the expired percent-heavy coprocess remained marked ready'
wait_for_deferred_reap
unset MBX_STALL_RESPONSE_MODE MBX_STALL_RESPONSE_MARKER
rm -f -- "$response_marker"

# The coprocess consumes the one render budget. Cleanup must be nonblocking and
# the coordinator must not start a fresh per-call process with a new timeout.
MBX_IPC_MODE=coprocess
MBX_IPC_TIMEOUT=.25
MBX_RENDER_TIMEOUT=.04
_mbx_engine_start || fail 'the stalling fixture did not complete its handshake'
_mbx_clock_now_us
started_us=$REPLY
_mbx_update_prompt 0 -
_mbx_clock_now_us
elapsed_us=$((REPLY - started_us))
((elapsed_us < 200000)) || fail 'the fallback chain exceeded one overall render deadline'
assert_eq 0 "${_MBX_ENGINE_READY:-missing}" \
    'a timed-out coprocess remained marked ready'
[[ ! -e $marker ]] || fail 'the coordinator granted per-call a second timeout budget'
[[ -n $PS1 ]] || fail 'the deadline path did not commit the builtin fallback'
wait_for_deferred_reap
unset MBX_STALL_PROMPT_MARKER

MBX_BIN=/bin/echo
MBX_IPC_MODE=per-call
MBX_RENDER_TIMEOUT=.25
MBX_DISABLE_RENDERER=0
_MBX_ENGINE_READY=0
_mbx_update_prompt 9 2500
assert_eq \
    "prompt --cwd $PWD --status 9 --flags $expected_flags --duration-ms 2500" \
    "$PS1" 'the prompt coordinator did not commit the per-call result'

ps1_writer_count=0
for bash_module in "$ROOT"/bash/*.bash; do
    while IFS= read -r source_line; do
        if [[ $source_line =~ ^[[:space:]]*PS1= ]]; then
            ((ps1_writer_count += 1))
            [[ ${bash_module##*/} == prompt.bash ]] || \
                fail "${bash_module##*/} writes PS1 outside the coordinator"
        fi
    done <"$bash_module"
done
assert_eq 1 "$ps1_writer_count" 'prompt.bash is not the sole PS1 writer'

# History text is sensitive. Bash modules must not grow an ad hoc debug-file
# channel that can bypass the standard command-text-free tracing boundary.
for bash_module in "$ROOT"/bash/*.bash; do
    [[ $(<"$bash_module") != *MBX_DBG* ]] || \
        fail "${bash_module##*/} contains the forbidden MBX_DBG channel"
done
[[ $(<"$ROOT/bash/history.bash") != *chmod* ]] || \
    fail 'history.bash must not spawn chmod on the prompt path'
[[ $(<"$ROOT/bash/completion.bash") != *set\ -euo\ pipefail* ]] || \
    fail 'completion.bash must not enable errexit/nounset/pipefail in the sourced module'

# Completion harness: default install defines no test fixtures (F-1).
source "$ROOT/bash/completion.bash"
_mbx_completion_install
_mbx_completion_install
assert_eq 1 "${_MBX_COMPLETION_INSTALLED:-missing}" \
    'completion install should be idempotent and leave the installed flag set'
declare -F mbx_comp_flag >/dev/null 2>&1 && \
    fail 'default completion install must not define mbx_comp_flag'
complete -p mbx_comp_flag >/dev/null 2>&1 && \
    fail 'default completion install must not bind complete -F on mbx_comp_flag'
declare -F mbx_comp_probe >/dev/null 2>&1 && \
    fail 'default completion install must not define mbx_comp_probe'
declare -F mbx_comp_rank >/dev/null 2>&1 && \
    fail 'default completion install must not define mbx_comp_rank'
declare -F mbx_comp_git >/dev/null 2>&1 && \
    fail 'default completion install must not define mbx_comp_git'

# Inspect-before-wrap: wrap a caller-defined -F; skip absent and non -F specs.
mbx_comp_wrap_src() { :; }
_mbx_comp_wrap_src_backend() {
    COMPREPLY=(mbx_wrap_candidate)
}
complete -F _mbx_comp_wrap_src_backend mbx_comp_wrap_src
_mbx_comp_wrap_existing_f mbx_comp_wrap_src || \
    fail 'wrap_existing_f should wrap a caller-defined -F spec'
complete -p mbx_comp_wrap_src | grep -Fq _mbx_comp_existing_adapter || \
    fail 'wrapped -F spec should use _mbx_comp_existing_adapter'
COMP_LINE='mbx_comp_wrap_src mbx_w'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(mbx_comp_wrap_src mbx_w)
COMP_CWORD=1
COMP_TYPE=9
COMP_KEY=$'\t'
_mbx_comp_existing_adapter
assert_eq mbx_wrap_candidate "${_MBX_COMP_LAST_REPLY:-}" \
    'existing -F wrap should preserve the original backend COMPREPLY'
mbx_comp_no_spec() { :; }
_mbx_comp_wrap_existing_f mbx_comp_no_spec && \
    fail 'wrap_existing_f should skip a command with no complete spec'
complete -p mbx_comp_no_spec >/dev/null 2>&1 && \
    fail 'skip of an unbound command must not install a complete spec'
mbx_comp_words() { :; }
complete -W 'alpha' mbx_comp_words
_mbx_comp_word_spec=$(complete -p mbx_comp_words)
_mbx_comp_wrap_existing_f mbx_comp_words && \
    fail 'wrap_existing_f should skip a non -F complete spec'
assert_eq "$_mbx_comp_word_spec" "$(complete -p mbx_comp_words)" \
    'non -F complete spec must be left unchanged'
unset -v _mbx_comp_word_spec

# Fixture opt-in for the existing probe/flag snapshot contract.
MBX_COMP_FIXTURES=1
_MBX_COMPLETION_INSTALLED=0
_mbx_completion_install
_mbx_completion_install
assert_eq 1 "${_MBX_COMPLETION_INSTALLED:-missing}" \
    'fixture opt-in install should be idempotent'
COMP_LINE='mbx_comp_probe mbx_co'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(mbx_comp_probe mbx_co)
COMP_CWORD=1
COMP_TYPE=9
COMP_KEY=$'\t'
_mbx_comp_probe_adapter
assert_eq 1 "${_MBX_COMP_SNAPPED:-missing}" 'adapter did not snapshot COMP_* state'
assert_eq "$COMP_LINE" "${_MBX_COMP_LINE:-}" 'snapshot COMP_LINE mismatch'
assert_eq "$COMP_POINT" "${_MBX_COMP_POINT:-missing}" 'snapshot COMP_POINT mismatch'
assert_eq 1 "${_MBX_COMP_CWORD:-missing}" 'snapshot COMP_CWORD mismatch'
assert_eq 2 "${#_MBX_COMP_WORDS[@]}" 'snapshot COMP_WORDS count mismatch'
assert_eq mbx_comp_candidate "${_MBX_COMP_LAST_REPLY:-}" \
    'adapter should preserve the backend COMPREPLY candidate'
assert_eq word "${_MBX_COMP_KINDS[0]:-}" \
    'probe adapter should record kind word for the candidate'
COMP_LINE='mbx_comp_flag --mbx-co'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(mbx_comp_flag --mbx-co)
COMP_CWORD=1
COMP_TYPE=9
COMP_KEY=$'\t'
_mbx_comp_flag_adapter
assert_eq --mbx-comp-flag "${_MBX_COMP_LAST_REPLY:-}" \
    'flag adapter should preserve the backend COMPREPLY candidate'
assert_eq flag "${_MBX_COMP_KINDS[0]:-}" \
    'flag adapter should record kind flag for the candidate'
assert_eq ${#COMPREPLY[@]} ${#_MBX_COMP_KINDS[@]} \
    'kinds array length should match COMPREPLY'
assert_eq ${#COMPREPLY[@]} ${#_MBX_COMP_DESCS[@]} \
    'descriptions array length should match COMPREPLY'
_mbx_comp_flag_nospace_adapter
assert_eq --mbx-comp-flag "${_MBX_COMP_LAST_REPLY:-}" \
    'nospace flag adapter should preserve the backend COMPREPLY candidate'
assert_eq 1 "${_MBX_COMPLETION_INSTALLED:-missing}" \
    'completion install should leave the installed flag set after flag adapters'
complete -p ls 2>/dev/null | grep -Fq '_mbx_comp' && \
    fail 'ls completion must not be wrapped by the MBX adapter'
complete -p printf 2>/dev/null | grep -Fq '_mbx_comp' && \
    fail 'printf completion must not be wrapped by the MBX adapter'
_mbx_comp_command_uses_flag_adapter mbx_comp_flag || \
    fail 'mbx_comp_flag should use the MBX flag adapter'
_mbx_comp_command_uses_flag_adapter mbx_comp_flag_nospace || \
    fail 'mbx_comp_flag_nospace should use the MBX flag adapter'

# COMP-003 metadata: sanitize and bound descriptions (K-4).
_mbx_comp_k4_backend() {
    local long desc
    long=$(printf 'a%.0s' {1..80})
    printf -v desc '%s $ ` %s' "$long" "$(printf '\001')"
    COMPREPLY=(candidate1)
    _MBX_COMP_BACKEND_DESCS=("$desc")
}
COMPREPLY=()
_MBX_COMP_KINDS=()
_MBX_COMP_DESCS=()
_mbx_comp_wrap_backend _mbx_comp_k4_backend
assert_eq 1 ${#COMPREPLY[@]} 'K-4 backend should return one candidate'
assert_eq 1 ${#_MBX_COMP_DESCS[@]} 'metadata should match COMPREPLY length'
[[ ${#_MBX_COMP_DESCS[0]} -le 64 ]] || \
    fail 'sanitized description exceeded the 64-character cap'
[[ ${_MBX_COMP_DESCS[0]} != *'$'* ]] || fail 'description still contains $'
[[ ${_MBX_COMP_DESCS[0]} != *'`'* ]] || fail 'description still contains backtick'
[[ ${_MBX_COMP_DESCS[0]} != *'\'* ]] || fail 'description still contains backslash'
[[ ${_MBX_COMP_DESCS[0]} != *$'\001'* ]] || fail 'description still contains a C0 byte'

# COMP-003 ranking: prefix scores and bound (R-2, R-3).
_mbx_comp_r2_backend() {
    COMPREPLY=(zzflag aaflag)
}
COMPREPLY=()
_MBX_COMP_SCORES=()
_MBX_COMP_ORDER=()
COMP_LINE='mbx_comp_rank aa'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(mbx_comp_rank aa)
COMP_CWORD=1
COMP_TYPE=9
COMP_KEY=$'\t'
_mbx_comp_wrap_backend _mbx_comp_r2_backend
assert_eq 2 ${#COMPREPLY[@]} 'R-2 backend should return two candidates'
assert_eq zzflag "${COMPREPLY[0]}" 'COMPREPLY order must stay stock'
assert_eq 2 ${#_MBX_COMP_SCORES[@]} 'scores length should match COMPREPLY'
assert_eq 2 ${#_MBX_COMP_ORDER[@]} 'order length should match COMPREPLY'
(( _MBX_COMP_SCORES[1] > _MBX_COMP_SCORES[0] )) || \
    fail 'aaflag should score higher than zzflag for prefix aa'
assert_eq 1 "${_MBX_COMP_ORDER[0]}" 'best-scoring aaflag index should sort first'
assert_eq aaflag "${_MBX_COMP_RANKED_REPLY:-}" \
    'ranked reply should prefer aaflag over stock COMPREPLY[0]'

# COMP-004 ranked accept: replace current word; refuse stale unrelated words.
READLINE_LINE='mbx_comp_rank aa'
READLINE_POINT=${#READLINE_LINE}
_mbx_comp_accept_ranked
assert_eq 'mbx_comp_rank aaflag' "$READLINE_LINE" \
    'ranked accept should replace the current word, not splice after it'
assert_eq ${#READLINE_LINE} "$READLINE_POINT" \
    'cursor should land after the replaced ranked candidate'
READLINE_LINE='echo ok'
READLINE_POINT=${#READLINE_LINE}
_MBX_COMP_RANKED_REPLY=aaflag
_mbx_comp_accept_ranked
assert_eq 'echo ok' "$READLINE_LINE" \
    'ranked accept must not mutate an unrelated current word'

_mbx_comp_r3_backend() {
    local i
    COMPREPLY=()
    for ((i = 0; i < 80; i++)); do
        COMPREPLY+=("mbx_comp_reply_$i")
    done
}
COMPREPLY=()
_MBX_COMP_SCORES=()
_MBX_COMP_ORDER=()
COMP_LINE='mbx_comp_rank mbx_comp_reply_0'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(mbx_comp_rank mbx_comp_reply_0)
COMP_CWORD=1
COMP_TYPE=9
COMP_KEY=$'\t'
_mbx_comp_wrap_backend _mbx_comp_r3_backend
assert_eq 80 ${#COMPREPLY[@]} 'R-3 backend should return 80 candidates'
assert_eq 80 ${#_MBX_COMP_SCORES[@]} 'scores length should match COMPREPLY'
assert_eq 80 ${#_MBX_COMP_ORDER[@]} 'order length should match COMPREPLY'
for ((i = 64; i < 80; i++)); do
    assert_eq 0 "${_MBX_COMP_SCORES[i]}" "reply index $i should keep score 0 beyond the 64-candidate bound"
done

# GIT-004: git candidate kinds stay additive; COMPREPLY order stays stock.
_mbx_comp_git_kinds_backend() {
    COMPREPLY=(zzref aaref --git-flag src/lib.rs)
}
COMPREPLY=()
_MBX_COMP_KINDS=()
_MBX_COMP_SCORES=()
_MBX_COMP_ORDER=()
COMP_LINE='mbx_comp_git aa'
COMP_POINT=${#COMP_LINE}
COMP_WORDS=(mbx_comp_git aa)
COMP_CWORD=1
COMP_TYPE=9
COMP_KEY=$'\t'
_mbx_comp_wrap_backend _mbx_comp_git_kinds_backend
assert_eq 4 ${#COMPREPLY[@]} 'git backend should return four candidates'
assert_eq zzref "${COMPREPLY[0]}" 'git COMPREPLY order must stay stock'
assert_eq ref "${_MBX_COMP_KINDS[0]:-}" 'zzref should be kind ref'
assert_eq ref "${_MBX_COMP_KINDS[1]:-}" 'aaref should be kind ref'
assert_eq flag "${_MBX_COMP_KINDS[2]:-}" '--git-flag should be kind flag'
assert_eq file "${_MBX_COMP_KINDS[3]:-}" 'src/lib.rs should be kind file'
assert_eq aaref "${_MBX_COMP_RANKED_REPLY:-}" \
    'prefix aa should rank aaref over stock zzref'

# History module contract: MBX2 record encoding, ACK decoding, exclusions,
# and the fork-free epoch-to-ISO conversion.
source "$ROOT/bash/history.bash"

_mbx_protocol_encode_history_record \
    5 'sess-1' 1 '-' 'echo hello' '/tmp' '2026-08-15T10:00:00Z' 0 '-' host user
assert_eq 'MBX2	5	RECORD	sess-1	1	-	echo hello	/tmp	2026-08-15T10:00:00Z	0	-	host	user' \
    "$REPLY" 'the MBX2 record encoder changed the wire format'
_mbx_protocol_decode_history_ack 5 $'MBX2\t5\tACK' || \
    fail 'a valid MBX2 ACK was rejected'
if _mbx_protocol_decode_history_ack 5 $'MBX2\t6\tACK'; then
    fail 'an MBX2 ACK with a mismatched request id was accepted'
fi
if _mbx_protocol_decode_history_ack 5 $'MBX1\t5\tACK'; then
    fail 'an MBX1 ACK was accepted as MBX2'
fi

MBX_HISTORY_EXCLUDE='git *:ssh *'
_mbx_history_excluded 'git status' || fail 'git exclusion did not match'
_mbx_history_excluded 'ssh host' || fail 'ssh exclusion did not match'
_mbx_history_excluded 'echo ok' && fail 'a non-excluded command was dropped'
unset MBX_HISTORY_EXCLUDE

_mbx_history_iso_utc 0
assert_eq '1970-01-01T00:00:00Z' "$REPLY" 'epoch zero did not convert to ISO'
_mbx_history_iso_utc 1735689600
assert_eq '2025-01-01T00:00:00Z' "$REPLY" 'the civil-date conversion is wrong'
_mbx_history_iso_utc 1786808647
assert_eq '2026-08-15T15:44:07Z' "$REPLY" 'the civil-date conversion drifted'

# Opt-in inline ghost (ADR 0010): suffix after POINT, never auto-executes.
source "$ROOT/bash/ghost.bash"
[[ $(<"$ROOT/bash/ghost.bash") != *set\ -euo\ pipefail* ]] || \
    fail 'ghost.bash must not enable errexit/nounset/pipefail in the sourced module'
if grep -Fq -- 'eval --' "$ROOT/bash/ghost.bash"; then
    fail 'ghost.bash must not eval the line; Enter uses accept-line (M-041)'
fi
ghost_stub_dir=$(mktemp -d)
cat >"$ghost_stub_dir/mbx" <<'EOF'
#!/bin/bash
printf '%s\n' 'echo MBX_GHST:alpha'
EOF
chmod +x "$ghost_stub_dir/mbx"
MBX_BIN=$ghost_stub_dir/mbx
MBX_GHOST=1
MBX_HISTORY=1
READLINE_LINE=
READLINE_POINT=0
READLINE_KEYSEQ=e
_mbx_ghost_self_insert
assert_eq 'echo MBX_GHST:alpha' "$READLINE_LINE" \
    'ghost should extend the typed prefix with a sidecar match'
assert_eq 1 "$READLINE_POINT" 'ghost should keep the cursor on the typed prefix'
assert_eq 1 "${_MBX_GHOST_HAS:-missing}" 'ghost should mark an active suffix'
_mbx_ghost_strip
assert_eq 'e' "$READLINE_LINE" 'ghost strip should restore the typed prefix'
assert_eq 0 "${_MBX_GHOST_HAS:-missing}" 'ghost strip should clear the suffix flag'
READLINE_LINE='echo MBX_GHST:alpha'
READLINE_POINT=1
_MBX_GHOST_HAS=1
_mbx_ghost_forward
assert_eq 19 "$READLINE_POINT" 'ghost accept should move the cursor to the end'
assert_eq 0 "${_MBX_GHOST_HAS:-missing}" 'ghost accept should clear the suffix flag'
READLINE_LINE='echo MBX_GHST:one two'
READLINE_POINT=15
_MBX_GHOST_HAS=1
_MBX_GHOST_POINT=15
_mbx_ghost_forward_word
assert_eq 17 "$READLINE_POINT" 'word-accept should land after the current word'
assert_eq 1 "${_MBX_GHOST_HAS:-missing}" 'word-accept should keep a remaining suffix'
assert_eq 17 "${_MBX_GHOST_POINT:-missing}" 'word-accept should advance the accepted prefix'
_mbx_ghost_forward_word
assert_eq 21 "$READLINE_POINT" 'second word-accept should reach the end'
assert_eq 0 "${_MBX_GHOST_HAS:-missing}" 'last word-accept should clear the suffix flag'
MBX_HISTORY=0
READLINE_LINE=
READLINE_POINT=0
READLINE_KEYSEQ=e
_MBX_GHOST_HAS=0
_mbx_ghost_self_insert
assert_eq 'e' "$READLINE_LINE" 'history-off ghost should insert the typed character only'
if _mbx_ghost_usable_match 'e' $'echo \033bad'; then
    fail 'ghost accepted a match containing an escape'
fi
cat >"$ghost_stub_dir/mbx" <<'EOF'
#!/bin/bash
printf '%s\n' 'echo first extra' 'echo second extra'
EOF
MBX_HISTORY=1
MBX_GHOST=1
READLINE_LINE=
READLINE_POINT=0
READLINE_KEYSEQ=e
_MBX_GHOST_HAS=0
_mbx_ghost_self_insert
assert_eq 'echo first extra' "$READLINE_LINE" \
    'ghost should show the newest prefix match first'
assert_eq 1 "$READLINE_POINT" 'ghost cycle should keep the cursor on the typed prefix'
assert_eq 2 "${#_MBX_GHOST_CANDIDATES[@]}" 'ghost should collect multiple prefix matches'
assert_eq 0 "${_MBX_GHOST_INDEX:-missing}" 'ghost should start on the newest match'
_mbx_ghost_cycle_next
assert_eq 'echo second extra' "$READLINE_LINE" \
    'ghost cycle next should show the older prefix match'
assert_eq 1 "$READLINE_POINT" 'ghost cycle next should restore the typed prefix point'
assert_eq 1 "${_MBX_GHOST_INDEX:-missing}" 'ghost cycle next should advance the index'
_mbx_ghost_cycle_next
assert_eq 'echo first extra' "$READLINE_LINE" \
    'ghost cycle next should wrap to the newest match'
assert_eq 0 "${_MBX_GHOST_INDEX:-missing}" 'ghost cycle next should wrap the index'
_mbx_ghost_cycle_prev
assert_eq 'echo second extra' "$READLINE_LINE" \
    'ghost cycle prev should wrap to the oldest collected match'
assert_eq 1 "${_MBX_GHOST_INDEX:-missing}" 'ghost cycle prev should wrap the index'
_mbx_ghost_quoted_keyseq '='
assert_eq '"="' "$REPLY" 'equals should use a quoted bind keyseq'
_mbx_ghost_quoted_keyseq '"'
assert_eq '"\""' "$REPLY" 'double-quote should use a Readline escaped keyseq'
_mbx_ghost_quoted_keyseq '\'
assert_eq '"\\"' "$REPLY" 'backslash should use a Readline escaped keyseq'
_mbx_ghost_quoted_keyseq '\C-h'
assert_eq '"\C-h"' "$REPLY" 'control keyseq should keep its Readline form'
cat >"$ghost_stub_dir/mbx" <<'EOF'
#!/bin/bash
printf '%s\n' 'echo foo=bar'
EOF
READLINE_LINE='echo foo'
READLINE_POINT=8
READLINE_KEYSEQ='='
_MBX_GHOST_HAS=0
_mbx_ghost_self_insert
assert_eq 'echo foo=bar' "$READLINE_LINE" \
    'ghost should extend a prefix that ends with equals'
assert_eq 9 "$READLINE_POINT" 'equals insert should keep the cursor on the typed prefix'
READLINE_LINE='echo MBX_GHST:alpha'
READLINE_POINT=15
_MBX_GHOST_HAS=1
_MBX_GHOST_POINT=15
_mbx_ghost_backward
assert_eq 'echo MBX_GHST:a' "$READLINE_LINE" \
    'ghost Left should restore the typed prefix without the suffix'
assert_eq 14 "$READLINE_POINT" 'ghost Left should move one character into the typed prefix'
assert_eq 0 "${_MBX_GHOST_HAS:-missing}" 'ghost Left should clear the suffix flag'
rm -rf "$ghost_stub_dir"
unset MBX_BIN MBX_GHOST MBX_HISTORY READLINE_LINE READLINE_POINT READLINE_KEYSEQ \
    _MBX_GHOST_HAS _MBX_GHOST_POINT _MBX_GHOST_INDEX _MBX_GHOST_TYPED_LEN \
    _MBX_GHOST_CANDIDATES

printf 'PASS: focused Bash module contracts\n'
