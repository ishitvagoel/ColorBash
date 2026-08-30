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

if _mbx_text_has_c0_or_del 'echo ok'; then
    fail 'printable text was treated as a C0/DEL payload'
fi
_mbx_text_has_c0_or_del $'echo \033hijack' || \
    fail 'ESC in command text must be treated as a C0 payload'
_mbx_text_has_c0_or_del $'echo \thit' || \
    fail 'TAB in command text must be treated as a C0 payload'
_mbx_text_has_c0_or_del $'echo \177hit' || \
    fail 'DEL in command text must be treated as a C0 payload'

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

# M-051: the session coprocess must ignore SIGINT and restore monitor/notify.
MBX_BIN=$MBX_TEST_BIN
MBX_IPC_MODE=coprocess
MBX_DISABLE_RENDERER=0
MBX_IPC_TIMEOUT=.25
set -m
set -b
_mbx_engine_start || fail 'M-051: coprocess did not become ready under monitor mode'
[[ $- == *m* ]] || fail 'M-051: engine start left monitor mode off'
[[ $- == *b* ]] || fail 'M-051: engine start left notify off'
engine_pid=${_MBX_ENGINE_CHILD_PID:-}
[[ $engine_pid =~ ^[1-9][0-9]*$ ]] || fail 'M-051: missing engine child pid'
jobs_out=$(jobs 2>/dev/null || true)
[[ $jobs_out != *_MBX_ENGINE_COPROC* ]] || \
    fail 'M-051: engine coprocess remained a monitored job'
kill -INT "$engine_pid"
_mbx_engine_ping || fail 'M-051: coprocess did not survive SIGINT'
kill -0 "$engine_pid" 2>/dev/null || fail 'M-051: engine pid exited after SIGINT'
_mbx_engine_stop
set +m
set +b
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
export MBX_STALL_SERVE_PROMPT_MARKER=$serve_prompt_marker

# What must hold is that the render *deadline* governs how long a near-limit
# request against a stalled coprocess can take. A single run against a
# hardcoded wall-clock ceiling cannot test that, because the run also carries a
# fixed per-version cost outside the deadline-governed section, and that cost
# differs several-fold across the Bash releases this project supports. Measured
# on one host at render timeouts of 50/100/200 ms, elapsed minus timeout was a
# flat ~121 ms on Bash 5.0 and a flat ~32 ms on Bash 5.2 — the deadline is
# honored exactly on both, but a fixed 200 ms ceiling (100 ms timeout plus
# 100 ms of allowance) fits only the faster one. That made the old assertion a
# benchmark of the host's Bash build rather than a check of the deadline, and
# it failed the Bash 5.0 CI leg for that reason alone.
#
# So measure the deadline directly: run the same stalled request at two
# timeouts and require the elapsed times to differ by the timeout difference.
# The per-version fixed cost is identical in both runs and cancels out. A
# deadline that is not honored cannot produce that difference — the stall is
# indefinite, so a broken deadline hangs for seconds, which the absolute
# ceiling below also catches.
measure_near_limit_prompt() {
    local timeout=$1
    rm -f -- "$marker" "$serve_prompt_marker"
    MBX_RENDER_TIMEOUT=$timeout
    _mbx_engine_start || fail 'the near-limit-PWD fixture did not complete its handshake'
    _mbx_clock_now_us
    local started=$REPLY
    PWD=$near_limit_logical_pwd _mbx_update_prompt 0 -
    _mbx_clock_now_us
    REPLY=$((REPLY - started))
}

# The shorter of the two runs stays at the original .10, never below it: the
# request is a full 64 KiB and on the slowest supported Bash in a container it
# needs that long just to reach the stalled peer. A .05 run measured the
# deadline correctly but never sent, which is a different case than this one is
# for.
measure_near_limit_prompt .10
near_limit_fast_us=$REPLY
# The stalled peer must be reachable and the deadline must not have been
# refreshed per call, on this run as much as the next.
[[ -e $serve_prompt_marker ]] || fail 'the fitting near-limit request was not sent'
[[ ! -e $marker ]] || fail 'near-limit rendering granted per-call a second budget'
wait_for_deferred_reap

measure_near_limit_prompt .20
near_limit_slow_us=$REPLY
near_limit_delta_us=$((near_limit_slow_us - near_limit_fast_us))
# Window measured, not guessed: the difference lands near the nominal 100000us
# on an idle host and compressed to ~68000us with every core saturated, so the
# lower bound has to clear that. A deadline that does not govern produces a
# delta of roughly zero (confirmed at -364us by sabotaging the fixture so both
# runs share one budget), which stays far outside this window.
((near_limit_delta_us > 30000 && near_limit_delta_us < 250000)) || \
    fail "the render deadline did not govern the near-limit request: a 100000us larger timeout changed elapsed time by ${near_limit_delta_us}us (${near_limit_fast_us}us then ${near_limit_slow_us}us)"
# Absolute ceiling: the stall never returns, so a deadline that fails to fire
# leaves this in the seconds. Generous enough for the slowest supported Bash,
# tight enough that an unbounded wait cannot pass.
((near_limit_fast_us < 1000000)) || \
    fail "near-limit request against a stalled coprocess was not bounded: ${near_limit_fast_us}us"
elapsed_us=$near_limit_slow_us
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
    if grep -Eq '^[[:space:]]*eval[[:space:]]' "$bash_module"; then
        fail "${bash_module##*/} contains eval (suggestions must never execute)"
    fi
done
[[ $(<"$ROOT/bash/history.bash") != *chmod* ]] || \
    fail 'history.bash must not spawn chmod on the prompt path'
[[ $(<"$ROOT/bash/completion.bash") != *set\ -euo\ pipefail* ]] || \
    fail 'completion.bash must not enable errexit/nounset/pipefail in the sourced module'

# User config is sourced only from an absolute readable file; env wins even if
# the file uses a bare export.
mbx_cfg_dir=$(mktemp -d "${TMPDIR:-/tmp}/mbx-cfg.XXXXXXXX")
printf '[[ ${MBX_HISTORY+x} ]] || export MBX_HISTORY=1\n' >"$mbx_cfg_dir/config.bash"
printf '[[ ${MBX_GHOST+x} ]] || export MBX_GHOST=1\n' >>"$mbx_cfg_dir/config.bash"
_MBX_USER_CONFIG_LOADED=0
unset MBX_HISTORY MBX_GHOST
MBX_CONFIG=$mbx_cfg_dir/config.bash
_mbx_load_user_config
assert_eq 1 "${MBX_HISTORY-}" 'user config should set history when unset'
assert_eq 1 "${MBX_GHOST-}" 'user config should set ghost when unset'
MBX_HISTORY=0
_MBX_USER_CONFIG_LOADED=0
_mbx_load_user_config
assert_eq 0 "$MBX_HISTORY" 'an existing env value must win over user config'
printf 'export MBX_HISTORY=1\nexport MBX_GHOST=1\n' >"$mbx_cfg_dir/config.bash"
MBX_HISTORY=0
unset MBX_GHOST
_MBX_USER_CONFIG_LOADED=0
_mbx_load_user_config
assert_eq 0 "$MBX_HISTORY" 'a bare export in user config must not clobber env'
assert_eq 1 "${MBX_GHOST-}" 'a bare export may set an unset MBX_* flag'
_MBX_USER_CONFIG_LOADED=0
unset MBX_HISTORY
MBX_CONFIG=relative/mbx/config.bash
_mbx_load_user_config
[[ -z ${MBX_HISTORY+x} ]] || fail 'relative MBX_CONFIG must not be sourced'
status_out=$(mbx_status)
[[ $status_out == *'config: relative/mbx/config.bash'* ]] || \
    fail "mbx_status should print the configured path: $status_out"
[[ $status_out == *mbx_configure* ]] || \
    fail 'mbx_status should mention mbx_configure'
[[ $status_out == *'duration:'* ]] || \
    fail "mbx_status should print duration: $status_out"
[[ $status_out == *'persist-bashrc:'* ]] || \
    fail "mbx_status should print persist-bashrc: $status_out"
declare -F mbx_configure >/dev/null 2>&1 || fail 'mbx_configure should be defined'
_MBX_ROOT=
mbx_configure --help >/dev/null 2>&1 && \
    fail 'mbx_configure without _MBX_ROOT should fail'

# D-1: mbx doctor reports every section and fails closed on a missing helper.
declare -F mbx_doctor >/dev/null 2>&1 || fail 'mbx_doctor should be defined'
MBX_BIN=/nonexistent/mbx
doctor_status=0
doctor_out=$(mbx_doctor) || doctor_status=$?
[[ $doctor_out == *'[FAIL]'*'MBX_BIN is unset or not executable'* ]] || \
    fail "mbx doctor should fail closed on a missing helper: $doctor_out"
((doctor_status != 0)) || fail 'mbx doctor must exit nonzero when a FAIL line was printed'
for doctor_section in Shell 'Terminal capability' Helper Configuration \
    'Keybinding collisions' 'History store'; do
    [[ $doctor_out == *"$doctor_section"* ]] || \
        fail "mbx doctor should print a $doctor_section section: $doctor_out"
done

# D-2: with a working helper, doctor reports OK for every helper check. The
# harness sources modules non-interactively, so the shell-interactivity FAIL
# is expected here and is not what this case is checking.
MBX_BIN=$MBX_TEST_BIN
doctor_out=$(mbx_doctor) || true
[[ $doctor_out == *'[OK]   live handshake: mbx/'* ]] || \
    fail "mbx doctor should report a live handshake: $doctor_out"
[[ $doctor_out == *'[OK]'*'is executable'* ]] || \
    fail "mbx doctor should report the helper as executable: $doctor_out"
[[ $doctor_out == *'[OK]   version: mbx '* ]] || \
    fail "mbx doctor should report the helper version: $doctor_out"

# D-3: MBX_GHOST=1 and MBX_HIGHLIGHT=1 together must be reported as a FAIL.
MBX_GHOST=1
MBX_HIGHLIGHT=1
doctor_status=0
doctor_out=$(mbx_doctor) || doctor_status=$?
[[ $doctor_out == *'[FAIL]'*'mutually exclusive'* ]] || \
    fail "mbx doctor should flag ghost+highlight as mutually exclusive: $doctor_out"
((doctor_status != 0)) || fail 'mbx doctor must exit nonzero when ghost+highlight collide'
unset MBX_GHOST MBX_HIGHLIGHT

# D-4: the collision report covers every chord MBX installs, not only the
# three opt-in features. An always-on installer that declined an occupied
# chord must be named along with its own override variable.
_MBX_SEARCH_BOUND=0
_MBX_SEARCH_RESTORE_BOUND=1
_MBX_EDITOR_INSERT_BOUND=0
_MBX_COMP_ACCEPT_BOUND=1
_MBX_COMP_CYCLE_NEXT_BOUND=1
_MBX_COMP_CYCLE_PREV_BOUND=1
doctor_out=$(mbx_doctor) || true
[[ $doctor_out == *'history-search insert (Ctrl-X h)'* ]] || \
    fail "mbx doctor should report the history-search chord: $doctor_out"
[[ $doctor_out == *'MBX_SEARCH_OVERRIDE=1'* ]] || \
    fail "mbx doctor should name MBX_SEARCH_OVERRIDE for a declined search chord: $doctor_out"
[[ $doctor_out == *'MBX_EDITOR_OVERRIDE=1'* ]] || \
    fail "mbx doctor should name MBX_EDITOR_OVERRIDE for a declined insert-token chord: $doctor_out"
[[ $doctor_out == *'[OK]   history-search restore (Ctrl-X l): chord bound'* ]] || \
    fail "mbx doctor should report a bound restore chord as OK: $doctor_out"
[[ $doctor_out != *'no MBX keystroke feature is installed'* ]] || \
    fail "mbx doctor must not claim nothing is installed when chords are bound: $doctor_out"
unset _MBX_SEARCH_BOUND _MBX_SEARCH_RESTORE_BOUND _MBX_EDITOR_INSERT_BOUND \
    _MBX_COMP_ACCEPT_BOUND _MBX_COMP_CYCLE_NEXT_BOUND _MBX_COMP_CYCLE_PREV_BOUND

# D-5: MBX_HISTORY=1 with a store whose path resolves but whose count fails is
# an unusable store and must be a FAIL, not a silently omitted row count.
doctor_stub_dir=$(mktemp -d)
cat >"$doctor_stub_dir/mbx" <<'EOF'
#!/bin/sh
case "$1 $2" in
    "history path") printf '%s\n' "/nonexistent/store.sqlite3" ;;
    "history count") exit 1 ;;
    *) printf 'mbx 0.0.0-test\n' ;;
esac
EOF
chmod +x "$doctor_stub_dir/mbx"
MBX_BIN=$doctor_stub_dir/mbx
MBX_HISTORY=1
doctor_status=0
doctor_out=$(mbx_doctor) || doctor_status=$?
[[ $doctor_out == *'[FAIL]'*'could not be read'* ]] || \
    fail "mbx doctor should fail when the history store cannot be read: $doctor_out"
((doctor_status != 0)) || \
    fail 'mbx doctor must exit nonzero when the history store is unreadable'
unset MBX_HISTORY
rm -rf "$doctor_stub_dir"

unset _MBX_ROOT MBX_CONFIG MBX_HISTORY MBX_GHOST
_MBX_USER_CONFIG_LOADED=1
rm -f "$mbx_cfg_dir/config.bash"
rmdir "$mbx_cfg_dir"

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

mbx_comp_wrap_opts() { :; }
_mbx_comp_wrap_opts_backend() {
    COMPREPLY=(mbx_opts_candidate)
}
complete -o nospace -P pre -S suf -X '!*.o' -F _mbx_comp_wrap_opts_backend \
    mbx_comp_wrap_opts
_mbx_comp_wrap_existing_f mbx_comp_wrap_opts || \
    fail 'wrap_existing_f should wrap a -F spec that has -P/-S/-X'
_mbx_comp_opts_spec=$(complete -p mbx_comp_wrap_opts)
[[ $_mbx_comp_opts_spec == *_mbx_comp_existing_adapter* ]] || \
    fail 'wrapped -P/-S/-X spec should use _mbx_comp_existing_adapter'
[[ $_mbx_comp_opts_spec == *-o*nospace* ]] || \
    fail 'wrapped spec should keep -o nospace'
[[ $_mbx_comp_opts_spec == *-P*pre* ]] || \
    fail 'wrapped spec should keep -P prefix'
[[ $_mbx_comp_opts_spec == *-S*suf* ]] || \
    fail 'wrapped spec should keep -S suffix'
[[ $_mbx_comp_opts_spec == *-X* ]] || \
    fail 'wrapped spec should keep -X filter'
unset -v _mbx_comp_opts_spec

mbx_comp_wrap_cfg() { :; }
_mbx_comp_wrap_cfg_backend() {
    COMPREPLY=(mbx_cfg_candidate)
}
complete -F _mbx_comp_wrap_cfg_backend mbx_comp_wrap_cfg
MBX_COMP_WRAP=mbx_comp_wrap_cfg
_MBX_COMPLETION_INSTALLED=0
_mbx_completion_install
complete -p mbx_comp_wrap_cfg | grep -Fq _mbx_comp_existing_adapter || \
    fail 'MBX_COMP_WRAP should wrap listed -F completers'
unset MBX_COMP_WRAP

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
assert_eq 2 ${#_MBX_COMP_RANKED_LIST[@]} 'ranked list should snapshot ordered candidates'
assert_eq aaflag "${_MBX_COMP_RANKED_LIST[0]}" 'ranked list head should match ranked reply'
assert_eq zzflag "${_MBX_COMP_RANKED_LIST[1]}" 'ranked list should keep zzflag after aaflag'

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
_mbx_comp_wrap_backend _mbx_comp_r2_backend
READLINE_LINE='echo aa'
READLINE_POINT=${#READLINE_LINE}
_mbx_comp_accept_ranked
assert_eq 'echo aa' "$READLINE_LINE" \
    'ranked accept must not replace a prefix-colliding word at a different offset'
_mbx_comp_cycle_next
assert_eq 'echo aa' "$READLINE_LINE" \
    'ranked cycle must not replace a prefix-colliding word at a different offset'

# COMP-004 ranked cycle: prefix inserts head; equal head rotates; unrelated is no-op.
_mbx_comp_wrap_backend _mbx_comp_r2_backend
READLINE_LINE='mbx_comp_rank aa'
READLINE_POINT=${#READLINE_LINE}
_mbx_comp_cycle_next
assert_eq 'mbx_comp_rank aaflag' "$READLINE_LINE" \
    'cycle-next on a prefix should insert the ranked head without rotating'
assert_eq aaflag "${_MBX_COMP_RANKED_REPLY:-}" \
    'prefix cycle-next must not rotate the ranked head'
assert_eq aaflag "${_MBX_COMP_RANKED_LIST[0]}" \
    'prefix cycle-next must not rotate the ranked list'
_mbx_comp_cycle_next
assert_eq 'mbx_comp_rank zzflag' "$READLINE_LINE" \
    'cycle-next from the ranked head should replace with the next candidate'
assert_eq zzflag "${_MBX_COMP_RANKED_REPLY:-}" \
    'cycle-next should update the ranked head to zzflag'
assert_eq zzflag "${_MBX_COMP_RANKED_LIST[0]}" \
    'cycle-next should rotate zzflag to the list head'
_mbx_comp_wrap_backend _mbx_comp_r2_backend
READLINE_LINE='mbx_comp_rank aaflag'
READLINE_POINT=${#READLINE_LINE}
_mbx_comp_cycle_prev
assert_eq 'mbx_comp_rank zzflag' "$READLINE_LINE" \
    'cycle-prev from the ranked head should wrap to the last candidate'
assert_eq zzflag "${_MBX_COMP_RANKED_REPLY:-}" \
    'cycle-prev should update the ranked head to zzflag'
_mbx_comp_wrap_backend _mbx_comp_r2_backend
READLINE_LINE='echo ok'
READLINE_POINT=${#READLINE_LINE}
_mbx_comp_cycle_next
assert_eq 'echo ok' "$READLINE_LINE" \
    'ranked cycle must not mutate an unrelated current word'
assert_eq aaflag "${_MBX_COMP_RANKED_REPLY:-}" \
    'unrelated cycle-next must not rotate the ranked head'

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
assert_eq 64 ${#_MBX_COMP_RANKED_LIST[@]} 'ranked cycle list should cap at 64'
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

# COMP-004 overlay: snapshot ranked rows and toggle visibility (OV-1).
MBX_COMP_OVERLAY=1
_mbx_comp_wrap_backend _mbx_comp_r2_backend
assert_eq 2 ${#_MBX_COMP_OVERLAY_CANDIDATES[@]} \
    'overlay snapshot should copy ranked candidates'
assert_eq aaflag "${_MBX_COMP_OVERLAY_CANDIDATES[0]:-}" \
    'overlay snapshot head should match ranked list'
assert_eq zzflag "${_MBX_COMP_OVERLAY_CANDIDATES[1]:-}" \
    'overlay snapshot should keep the second ranked row'
_MBX_COMP_OVERLAY_VISIBLE=0
_mbx_comp_overlay_toggle
(( _MBX_COMP_OVERLAY_VISIBLE == 1 )) || fail 'overlay toggle should show the list'
_mbx_comp_overlay_toggle
(( _MBX_COMP_OVERLAY_VISIBLE == 0 )) || fail 'overlay toggle should hide the list'
_mbx_comp_wrap_backend _mbx_comp_r2_backend
_MBX_COMP_OVERLAY_VISIBLE=1
_MBX_COMP_OVERLAY_LINES=2
_mbx_comp_cycle_next
assert_eq 1 "${_MBX_COMP_OVERLAY_INDEX:-0}" \
    'overlay cycle-next should advance the selection index'
_mbx_comp_overlay_dismiss
(( _MBX_COMP_OVERLAY_VISIBLE == 0 )) || fail 'overlay dismiss should hide the list'

MBX_COMP_OVERLAY=1
_mbx_comp_wrap_backend _mbx_comp_r3_backend
assert_eq 8 ${#_MBX_COMP_OVERLAY_CANDIDATES[@]} \
    'overlay snapshot should cap ranked rows at eight'
assert_eq mbx_comp_reply_0 "${_MBX_COMP_OVERLAY_CANDIDATES[0]:-}" \
    'overlay snapshot head should match the ranked list'
assert_eq mbx_comp_reply_7 "${_MBX_COMP_OVERLAY_CANDIDATES[7]:-}" \
    'overlay snapshot should keep the eighth ranked row'

# OV-2 (M-065): the overlay must never draw more rows than the terminal can
# hold under the prompt. Reserving rows keeps the saved cursor valid, but
# reserving more rows than exist scrolls the prompt off the top entirely, so
# the draw is capped at LINES-2 — the prompt's own row plus one line of
# context.
saved_lines=${LINES:-}
LINES=6
_mbx_comp_overlay_capacity
assert_eq 4 "$REPLY" 'a six-row terminal should allow four overlay rows'
LINES=24
_mbx_comp_overlay_capacity
assert_eq 22 "$REPLY" 'a 24-row terminal should allow 22 overlay rows'
LINES=2
_mbx_comp_overlay_capacity
assert_eq 0 "$REPLY" 'a two-row terminal leaves no room for the overlay'
LINES=1
_mbx_comp_overlay_capacity
assert_eq 0 "$REPLY" 'capacity must clamp at zero rather than go negative'
LINES=not-a-number
_mbx_comp_overlay_capacity
assert_eq 22 "$REPLY" 'a nonsensical LINES should fall back to 24 rows, not disable the overlay'
unset LINES
_mbx_comp_overlay_capacity
assert_eq 22 "$REPLY" 'an unset LINES should fall back to 24 rows'
if [[ -n $saved_lines ]]; then
    LINES=$saved_lines
fi
unset saved_lines
_MBX_COMP_OVERLAY_VISIBLE=1
_MBX_COMP_OVERLAY_LINES=8
_MBX_COMP_OVERLAY_INDEX=0
_mbx_comp_cycle_next
(( _MBX_COMP_OVERLAY_INDEX < 8 )) || fail 'overlay cycle must stay within the eight-row window'
_mbx_comp_sanitize_display $'aa\033flag'
assert_eq 'aa?flag' "$REPLY" 'overlay display sanitize should replace ESC bytes'
unset MBX_COMP_OVERLAY

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

_mbx_protocol_encode_history_query 9 42 prefix 'git st' 5
assert_eq 'MBX2	9	QUERY	42	prefix	git st	5' "$REPLY" \
    'the MBX2 QUERY encoder changed the wire format'
_mbx_protocol_encode_history_query 9 1 failed '-' 3
assert_eq 'MBX2	9	QUERY	1	failed	-	3' "$REPLY" \
    'failed QUERY must keep a literal dash text field'
_mbx_protocol_encode_history_cancel 11 99
assert_eq 'MBX2	11	CANCEL	99' "$REPLY" \
    'the MBX2 CANCEL encoder changed the wire format'

mbx_query_cmds=()
_mbx_protocol_decode_history_result 9 $'MBX2\t9\tRESULT\t42\t2\tgit%20status\techo%09x' mbx_query_cmds || \
    fail 'a valid MBX2 RESULT was rejected'
assert_eq 42 "$REPLY" 'RESULT decode must return the generation in REPLY'
assert_eq 2 ${#mbx_query_cmds[@]} 'RESULT decode must fill the destination array'
assert_eq 'git status' "${mbx_query_cmds[0]}" 'RESULT command unescape drifted'
assert_eq $'echo\tx' "${mbx_query_cmds[1]}" 'RESULT tab unescape drifted'
if _mbx_protocol_decode_history_result 8 $'MBX2\t9\tRESULT\t1\t1\techo' mbx_query_cmds; then
    fail 'RESULT decode accepted a mismatched request id'
fi
_mbx_protocol_parse_history_result $'MBX2\t9\tRESULT\t1\t1\techo' mbx_query_cmds || \
    fail 'RESULT parse rejected a well-formed frame with a different request id'
assert_eq 1 "$REPLY" 'RESULT parse must return the generation without an id match'
assert_eq echo "${mbx_query_cmds[0]}" 'RESULT parse must still unescape commands'
if _mbx_protocol_decode_history_result 9 $'MBX2\t9\tRESULT\t42\t1' mbx_query_cmds; then
    fail 'RESULT with a short field count was accepted'
fi
if _mbx_protocol_decode_history_result 9 $'MBX2\t9\tRESULT\t42\t1\ta\textra' mbx_query_cmds; then
    fail 'RESULT with a trailing extra field was accepted'
fi
_mbx_protocol_decode_history_error 9 $'MBX2\t9\tERROR\tinvalid' || \
    fail 'a valid MBX2 ERROR was rejected'
assert_eq invalid "$REPLY" 'ERROR decode must return the typed kind'

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
_mbx_ghost_quoted_keyseq '$'
assert_eq '"$"' "$REPLY" 'dollar should use a quoted bind keyseq'
_mbx_ghost_quoted_keyseq '`'
assert_eq '"`"' "$REPLY" 'backtick should match the bind -p quoted keyseq'
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
READLINE_LINE='echo MBX_GHST:alpha'
READLINE_POINT=15
_MBX_GHOST_HAS=1
_MBX_GHOST_POINT=15
_mbx_ghost_beginning
assert_eq 'echo MBX_GHST:a' "$READLINE_LINE" \
    'ghost Home should restore the typed prefix without the suffix'
assert_eq 0 "$READLINE_POINT" 'ghost Home should move to the beginning of the typed prefix'
assert_eq 0 "${_MBX_GHOST_HAS:-missing}" 'ghost Home should clear the suffix flag'
READLINE_LINE='echo MBX_GHST:one two'
READLINE_POINT=17
_MBX_GHOST_HAS=1
_MBX_GHOST_POINT=17
_mbx_ghost_backward_word
assert_eq 'echo MBX_GHST:one' "$READLINE_LINE" \
    'ghost backward-word should restore the typed prefix without the suffix'
assert_eq 14 "$READLINE_POINT" 'ghost backward-word should land before the remaining word'
assert_eq 0 "${_MBX_GHOST_HAS:-missing}" 'ghost backward-word should clear the suffix flag'
history -c
history -s 'echo MBX_GHST:alpha'
history -s 'echo MBX_GHST:beta'
READLINE_LINE='echo MBX_GHST:alpha'
READLINE_POINT=15
_MBX_GHOST_HAS=1
_MBX_GHOST_POINT=15
_MBX_GHOST_HIST_OFFSET=0
_MBX_GHOST_HIST_CURRENT=
_mbx_ghost_previous_history
assert_eq 'echo MBX_GHST:beta' "$READLINE_LINE" \
    'ghost Up should load the newest history row after stripping a suffix'
assert_eq 1 "${_MBX_GHOST_HIST_OFFSET:-missing}" 'ghost Up should advance the history offset'
assert_eq 'echo MBX_GHST:a' "${_MBX_GHOST_HIST_CURRENT-}" \
    'ghost Up should remember the stripped typed prefix for Down'
_mbx_ghost_next_history
assert_eq 'echo MBX_GHST:a' "$READLINE_LINE" \
    'ghost Down should restore the remembered typed prefix'
assert_eq 0 "${_MBX_GHOST_HIST_OFFSET:-missing}" 'ghost Down should clear the history offset'
history -c
history -s 'echo MBX_GHST:alpha'
history -s 'echo MBX_GHST:beta'
READLINE_LINE='echo MBX_GHST:a'
READLINE_POINT=15
_MBX_GHOST_HAS=0
_MBX_GHOST_HIST_OFFSET=0
_MBX_GHOST_HIST_CURRENT=
_mbx_ghost_previous_history
_mbx_ghost_previous_history
assert_eq 'echo MBX_GHST:alpha' "$READLINE_LINE" \
    'ghost Up twice should load the older history row'
assert_eq 2 "${_MBX_GHOST_HIST_OFFSET:-missing}" 'ghost Up twice should set offset 2'
_mbx_ghost_next_history
assert_eq 'echo MBX_GHST:beta' "$READLINE_LINE" \
    'ghost Down from offset 2 should load the newer history row'
assert_eq 1 "${_MBX_GHOST_HIST_OFFSET:-missing}" 'ghost Down should decrement the history offset'
history -c
for history_n in $(seq 1 12); do
    history -s "echo MBX_GHST:$history_n"
done
_mbx_ghost_history_entry 1 || fail 'ghost history parse should accept a two-digit list number'
assert_eq 'echo MBX_GHST:12' "$REPLY" \
    'ghost Up must strip the full history list number, not only the first digit (M-047)'
_MBX_GHOST_DELETE_KEYSEQ='\C-x\C-d'
_MBX_GHOST_ACCEPT_KEYSEQ='\C-x\C-m'
_mbx_ghost_enter_delete_macro 4
macro_four=$REPLY
[[ $macro_four == *'\C-x\C-d'* ]] || fail 'Enter macro should use reserved delete-char helper'
[[ $macro_four == *kill-line* ]] && fail 'Enter macro must not reference kill-line'
_mbx_ghost_enter_delete_macro 2
macro_two=$REPLY
(( ${#macro_four} - ${#macro_two} == 16 )) || \
    fail 'Enter macro length should track suffix byte count'
_MBX_GHOST_BOUND=1
_MBX_GHOST_WRAP_CTRL_J=0
_MBX_GHOST_VI_BOUND=0
_MBX_GHOST_ENTER_ARMED=0
ghost_macro=
_mbx_ghost_arm_enter_keymap() {
    ghost_macro=$3
    return 0
}
_mbx_ghost_disarm_enter_keymap() {
    return 0
}
_mbx_ghost_show 'echo MBX_GHST:alpha' 'echo MBX_GHST:a'
assert_eq 1 "${_MBX_GHOST_ENTER_ARMED:-missing}" 'ghost show should arm Enter for an active suffix'
macro_after_alpha=$ghost_macro
_mbx_ghost_show 'echo MBX_GHST:ab' 'echo MBX_GHST:a'
(( ${#ghost_macro} < ${#macro_after_alpha} )) || \
    fail 'ghost show should rebuild Enter macro when suffix length changes'
_MBX_GHOST_ENTER_ARMED=1
_MBX_GHOST_VI_BOUND=1
_MBX_GHOST_WRAP_CTRL_J=0
_MBX_GHOST_VI_WRAP_CTRL_J=1
_mbx_ghost_disarm_enter_keymap() {
    if [[ $1 == emacs ]]; then
        return 0
    fi
    return 1
}
_mbx_ghost_disarm_enter || true
assert_eq 0 "${_MBX_GHOST_ENTER_ARMED:-missing}" \
    'partial keymap disarm must still clear ENTER_ARMED (M-044)'
source "$ROOT/bash/ghost.bash"
set -m
_mbx_ghost_query 'e' || true
[[ $- == *m* ]] || fail 'ghost query must restore monitor mode after a lookup'
set +m

_MBX_GHOST_GENERATION=2
mbx_stale_cmds=('echo MBX_GHST:alpha')
if _mbx_ghost_accept_commands 1 'echo MBX_GHST:a' 8 mbx_stale_cmds; then
    fail 'a stale QUERY generation was applied to ghost candidates'
fi
assert_eq 0 ${#_MBX_GHOST_CANDIDATES[@]} \
    'stale RESULT must not populate ghost candidates'
_mbx_ghost_accept_commands 2 'echo MBX_GHST:a' 8 mbx_stale_cmds || \
    fail 'the current QUERY generation was rejected'
assert_eq 'echo MBX_GHST:alpha' "${_MBX_GHOST_CANDIDATES[0]-}" \
    'current-generation RESULT must keep the prefix match'

rm -rf "$ghost_stub_dir"
unset MBX_BIN MBX_GHOST MBX_HISTORY READLINE_LINE READLINE_POINT READLINE_KEYSEQ \
    _MBX_GHOST_HAS _MBX_GHOST_POINT _MBX_GHOST_INDEX _MBX_GHOST_TYPED_LEN \
    _MBX_GHOST_CANDIDATES _MBX_GHOST_GENERATION

# Explicit history-search bind -x (ADR 0009): no-op unless MBX_HISTORY=1,
# replace the whole line with the first helper line, never enable errexit.
source "$ROOT/bash/search.bash"
[[ $(<"$ROOT/bash/search.bash") != *set\ -euo\ pipefail* ]] || \
    fail 'search.bash must not enable errexit/nounset/pipefail in the sourced module'
_mbx_search_install
_mbx_search_install
assert_eq 1 "${_MBX_SEARCH_INSTALLED:-missing}" \
    'search install should be idempotent and leave the installed flag set'

READLINE_LINE='keep-me'
READLINE_POINT=7
unset MBX_HISTORY || true
_mbx_search_insert
assert_eq 'keep-me' "$READLINE_LINE" \
    'search must no-op when MBX_HISTORY is unset'

search_stub_dir=$(mktemp -d)
cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
printf '%s\n' "printf 'MBX_SRCH:hit'"
EOF
chmod +x "$search_stub_dir/mbx"
MBX_HISTORY=1
MBX_BIN=$search_stub_dir/mbx
MBX_SEARCH_TIMEOUT=1.0
READLINE_LINE='printf'
READLINE_POINT=6
_mbx_search_insert
assert_eq "printf 'MBX_SRCH:hit'" "$READLINE_LINE" \
    'search should replace the line with the helper match'
assert_eq 21 "$READLINE_POINT" 'search should move the cursor to the end of the match'

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
printf '%s\n' "$(printf 'echo \033hijack')"
printf '%s\n' 'echo MBX_HRD:ok'
EOF
READLINE_LINE='echo'
READLINE_POINT=4
_mbx_search_insert
assert_eq 'echo MBX_HRD:ok' "$READLINE_LINE" \
    'search should skip C0 matches and insert the next clean line'
cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
printf '%s\n' "$(printf 'echo \033only')"
EOF
READLINE_LINE='keep-controls'
READLINE_POINT=13
_mbx_search_insert
assert_eq 'keep-controls' "$READLINE_LINE" \
    'search should leave the line unchanged when every match contains C0'
cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
printf '%s\n' "printf 'MBX_SRCH:hit'"
EOF

set -m
_mbx_search_helper 8 history search prefix printf --limit 8 || true
[[ $- == *m* ]] || fail 'search helper must restore monitor mode after a lookup (M-049)'
set +m

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
printf '%s\n' "match-one"
printf '%s\n' "match-two"
EOF
READLINE_LINE='q'
READLINE_POINT=1
_mbx_search_insert
assert_eq 'match-one' "$READLINE_LINE" 'first chord should insert the first helper line'
_mbx_search_insert
assert_eq 'match-two' "$READLINE_LINE" 'second chord should cycle to the next match'
_mbx_search_insert
assert_eq 'match-one' "$READLINE_LINE" 'third chord should wrap to the first match'
_mbx_search_restore
assert_eq 'q' "$READLINE_LINE" 'restore after cycling should put back the typed query'
assert_eq 1 "$READLINE_POINT" 'restore should put the cursor back on the typed query'
assert_eq 0 "${#_MBX_SEARCH_MATCHES[@]}" 'restore should drop the snapshot'
assert_eq 0 "${_MBX_SEARCH_HAS_ORIGINAL:-missing}" 'restore should drop the original line'

READLINE_LINE='keep-me'
READLINE_POINT=7
_mbx_search_restore
assert_eq 'keep-me' "$READLINE_LINE" \
    'restore with no snapshot should leave the line unchanged'

READLINE_LINE='q'
READLINE_POINT=1
_mbx_search_insert
assert_eq 'match-one' "$READLINE_LINE" 'search should insert after a prior restore'
_mbx_search_restore
assert_eq 'q' "$READLINE_LINE" 'restore after a fresh insert should put back the typed query'

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
exit 0
EOF
READLINE_LINE='q'
READLINE_POINT=1
_mbx_search_insert
assert_eq 'q' "$READLINE_LINE" 'a failed search should leave the typed line'
_mbx_search_restore
assert_eq 'q' "$READLINE_LINE" \
    'restore after a failed search should not revive a stale original'

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
printf '%s\n' "match-one"
printf '%s\n' "match-two"
EOF
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'match-one' "$READLINE_LINE" 'empty-line search should insert the first recent row'
_mbx_search_restore
assert_eq '' "$READLINE_LINE" 'restore should put back an empty typed line'
assert_eq 0 "$READLINE_POINT" 'restore of an empty line should put the cursor at 0'
_mbx_search_clear
assert_eq 0 "${#_MBX_SEARCH_MATCHES[@]}" 'search_clear should drop the snapshot'
assert_eq 0 "${_MBX_SEARCH_HAS_ORIGINAL:-missing}" 'search_clear should drop the original line'

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
case " $* " in
    *" search prefix "*" --cwd "*) printf '%s\n' "cwd-prefix-hit" ;;
    *" search prefix "*) printf '%s\n' "global-prefix-hit" ;;
    *) printf '%s\n' "other-hit" ;;
esac
EOF
READLINE_LINE='echo'
READLINE_POINT=4
_mbx_search_insert
assert_eq 'cwd-prefix-hit' "$READLINE_LINE" \
    'prefix search should prefer cwd matches when PWD is set'
_mbx_search_clear
MBX_SEARCH_CWD=0
READLINE_LINE='echo'
READLINE_POINT=4
_mbx_search_insert
assert_eq 'global-prefix-hit' "$READLINE_LINE" \
    'MBX_SEARCH_CWD=0 should use global prefix'
unset MBX_SEARCH_CWD
_mbx_search_clear

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
case " $* " in
    *" search failed "*) printf '%s\n' "failed-hit" ;;
    *" search cwd "*) printf '%s\n' "cwd-hit" ;;
    *" search recent "*) printf '%s\n' "recent-hit" ;;
    *) printf '%s\n' "other-hit" ;;
esac
EOF
unset MBX_SEARCH_FAILED
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'cwd-hit' "$READLINE_LINE" \
    'default empty-line search should still prefer cwd over failed'
_mbx_search_clear
MBX_SEARCH_FAILED=1
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'failed-hit' "$READLINE_LINE" \
    'MBX_SEARCH_FAILED=1 empty-line search should prefer failed rows'
unset MBX_SEARCH_FAILED
_mbx_search_clear

cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
case " $* " in
    *" search failed "*) ;;
    *" search cwd "*) printf '%s\n' "cwd-hit" ;;
    *" search recent "*) printf '%s\n' "recent-hit" ;;
    *) printf '%s\n' "other-hit" ;;
esac
EOF
MBX_SEARCH_FAILED=1
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'cwd-hit' "$READLINE_LINE" \
    'MBX_SEARCH_FAILED=1 with no failed rows should fall through to cwd'
unset MBX_SEARCH_FAILED
_mbx_search_clear

# R-1: MBX_SEARCH_REPO=1 resolves the root via `mbx repo root`, then prefers
# repo-scoped rows over cwd (PTY evidence in
# crates/pty/tests/history_search.rs: empty_line_inserts_repo_when_opt_in).
cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
case " $* " in
    *" repo root "*) printf '%s\n' "/fake/repo/root" ;;
    *" search repo "*) printf '%s\n' "repo-hit" ;;
    *" search cwd "*) printf '%s\n' "cwd-hit" ;;
    *" search recent "*) printf '%s\n' "recent-hit" ;;
    *) printf '%s\n' "other-hit" ;;
esac
EOF
MBX_SEARCH_REPO=1
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'repo-hit' "$READLINE_LINE" \
    'MBX_SEARCH_REPO=1 empty-line search should prefer repo-scoped rows'
unset MBX_SEARCH_REPO
_mbx_search_clear

# R-2: a resolvable-but-empty or failed repo root falls through to cwd
# rather than failing the whole lookup closed.
cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
case " $* " in
    *" repo root "*) exit 1 ;;
    *" search cwd "*) printf '%s\n' "cwd-hit" ;;
    *" search recent "*) printf '%s\n' "recent-hit" ;;
    *) printf '%s\n' "other-hit" ;;
esac
EOF
MBX_SEARCH_REPO=1
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'cwd-hit' "$READLINE_LINE" \
    'MBX_SEARCH_REPO=1 with no repo root should fall through to cwd'
unset MBX_SEARCH_REPO
_mbx_search_clear

# R-3: a helper that prints a plausible root but exits nonzero (a killed or
# timed-out child can leave a partial first line behind) must not be trusted;
# the lookup falls through to cwd rather than scoping to a half-read path.
cat >"$search_stub_dir/mbx" <<'EOF'
#!/bin/sh
case " $* " in
    *" repo root "*) printf '%s\n' "/fake/repo/root"; exit 1 ;;
    *" search repo "*) printf '%s\n' "repo-hit" ;;
    *" search cwd "*) printf '%s\n' "cwd-hit" ;;
    *" search recent "*) printf '%s\n' "recent-hit" ;;
    *) printf '%s\n' "other-hit" ;;
esac
EOF
MBX_SEARCH_REPO=1
READLINE_LINE=
READLINE_POINT=0
_mbx_search_insert
assert_eq 'cwd-hit' "$READLINE_LINE" \
    'a repo root printed by a failing helper must not be used'
unset MBX_SEARCH_REPO
_mbx_search_clear

MBX_HISTORY=0
READLINE_LINE='keep-me'
READLINE_POINT=7
_mbx_search_insert
assert_eq 'keep-me' "$READLINE_LINE" \
    'search must no-op when MBX_HISTORY is not 1'
_MBX_SEARCH_HAS_ORIGINAL=1
_MBX_SEARCH_ORIGINAL='secret'
_MBX_SEARCH_ORIGINAL_POINT=0
_mbx_search_restore
assert_eq 'keep-me' "$READLINE_LINE" \
    'restore must no-op when MBX_HISTORY is not 1'

MBX_HISTORY=1
MBX_BIN=/nonexistent/mbx-search-helper
READLINE_LINE='keep-me'
_mbx_search_insert
assert_eq 'keep-me' "$READLINE_LINE" \
    'search must no-op when the helper is missing'
rm -rf "$search_stub_dir"
unset MBX_BIN MBX_HISTORY MBX_SEARCH_TIMEOUT MBX_SEARCH_FAILED READLINE_LINE READLINE_POINT \
    _MBX_SEARCH_INSTALLED _MBX_SEARCH_MATCHES _MBX_SEARCH_INDEX \
    _MBX_SEARCH_ORIGINAL _MBX_SEARCH_ORIGINAL_POINT _MBX_SEARCH_HAS_ORIGINAL

source "$ROOT/bash/highlight.bash"
[[ $(<"$ROOT/bash/highlight.bash") != *set\ -euo\ pipefail* ]] || \
    fail 'highlight.bash must not enable errexit/nounset/pipefail in the sourced module'
_mbx_highlight_strip_line $'echo \033[31mhi\033[0m'
assert_eq 'echo hi' "$REPLY" 'highlight strip should remove markers and SGR'
_mbx_highlight_strip_line $'\001\033[32m\002ok\001\033[0m\002'
assert_eq 'ok' "$REPLY" 'highlight strip should keep plain bytes between markers'

highlight_stub_dir=$(mktemp -d)
cat >"$highlight_stub_dir/mbx" <<'EOF'
#!/bin/sh
if [ "$1" = highlight ]; then
    shift
    plain=
    point=0
    while [ $# -gt 0 ]; do
        case "$1" in
            --point) point=$2; shift 2 ;;
            --color) shift 2 ;;
            *) plain="$plain${plain:+ }$1"; shift ;;
        esac
    done
    printf '%s\n' "$(printf '\001\033[31m\002%s\001\033[0m\002' "$plain")"
    printf '%s\n' "$((point + 7))"
fi
EOF
chmod +x "$highlight_stub_dir/mbx"
MBX_BIN=$highlight_stub_dir/mbx
MBX_HIGHLIGHT=1
_MBX_HIGHLIGHT_PLAIN='echo hi'
_MBX_HIGHLIGHT_POINT=7
_MBX_HIGHLIGHT_ACTIVE=0
_mbx_highlight_refresh
assert_eq $'\001\033[31m\002echo hi\001\033[0m\002' "$READLINE_LINE" \
    'highlight refresh should install the styled helper line'
assert_eq 14 "$READLINE_POINT" 'highlight refresh should map the styled cursor'

# H-2: strip-then-compare accepts markers only when the stripped bytes match plain.
_MBX_HIGHLIGHT_PLAIN='echo hi'
styled_ok=$'\001\033[31m\002echo hi'
_mbx_highlight_validate_styled "$styled_ok" || \
    fail 'styled stub with markers should pass strip-then-compare'
styled_bad=$'\001\033[31m\002echo hiX'
_mbx_highlight_validate_styled "$styled_bad" && \
    fail 'styled stub with extra escape should be rejected'
_MBX_HIGHLIGHT_PLAIN='echo hi'
_MBX_HIGHLIGHT_POINT=7
_MBX_HIGHLIGHT_ACTIVE=0
set -m
_mbx_highlight_refresh || fail 'highlight refresh should succeed with the stub helper'
[[ $- == *m* ]] || fail 'highlight helper must restore monitor mode after a lookup (H-5)'
set +m
(( _MBX_HIGHLIGHT_ACTIVE == 1 )) || fail 'highlight refresh should activate styled mode'
_mbx_highlight_disarm_enter || true
_MBX_HIGHLIGHT_ACTIVE=0

# H-6: occupied bindings refuse overwrite unless override is set.
_mbx_user_hl_occupy() { :; }
bind -x '"z": _mbx_user_hl_occupy' 2>/dev/null || true
_MBX_HIGHLIGHT_INSTALLED=0
_MBX_HIGHLIGHT_BOUND=0
MBX_HIGHLIGHT=1
_mbx_highlight_install
bind -X 2>/dev/null | grep -Fq '_mbx_user_hl_occupy' || \
    fail 'occupied printable must not be overwritten without override'
bind -X 2>/dev/null | grep -Fq "_mbx_highlight_self_insert z" && \
    fail 'highlight must not steal an occupied printable binding'
bind -u z 2>/dev/null || true
bind -x '"\C-m": _mbx_user_hl_occupy' 2>/dev/null || true
_MBX_HIGHLIGHT_INSTALLED=0
_MBX_HIGHLIGHT_BOUND=0
_mbx_highlight_install
(( _MBX_HIGHLIGHT_BOUND == 0 )) || \
    fail 'occupied Enter should refuse highlight install when Enter cannot arm'
bind -u '"\C-m"' 2>/dev/null || true
unset -f _mbx_user_hl_occupy 2>/dev/null || true

# HLT-003 S-4: real helper corpus strip-round-trip in plain (non-tty) mode.
highlight_hostile_corpus=(
    'if echo "$HOME"; then true; fi # note'
    'cmd `whoami` $(id) ${HOME}'
    "printf '%s\\n' 'test\$'\\\`\\\\'"
    'echo "unclosed'
    "echo 'unclosed"
    'git commit -m "'\''; rm -rf /"'
    '100%_done'
    'ls /tmp/中文/café'
    'export PATH=/usr/bin:$PATH'
    '# comment only'
    'a=b c=d'
)
MBX_BIN=$MBX_TEST_BIN
MBX_HIGHLIGHT=1
for highlight_row in "${highlight_hostile_corpus[@]}"; do
    _MBX_HIGHLIGHT_PLAIN=$highlight_row
    _MBX_HIGHLIGHT_POINT=${#highlight_row}
    _MBX_HIGHLIGHT_ACTIVE=0
    _mbx_highlight_refresh || fail 'highlight refresh failed for hostile corpus row'
    _mbx_highlight_strip_line "$READLINE_LINE"
    assert_eq "$highlight_row" "$REPLY" \
        'helper corpus strip must round-trip exact bytes'
done

# HLT-003 P-2: wrapped self-insert refuses C0 bytes.
_MBX_HIGHLIGHT_PLAIN='echo keep'
_MBX_HIGHLIGHT_POINT=9
_MBX_HIGHLIGHT_ACTIVE=0
_mbx_highlight_self_insert $'\x01'
assert_eq 'echo keep' "$_MBX_HIGHLIGHT_PLAIN" \
    'highlight self-insert must refuse C0 bytes'

# H-6: MBX_HIGHLIGHT=1 and MBX_HISTORY=1 can both be on (only ghost and
# highlight are mutually exclusive), so a history RECORD's ACK can still be
# queued on the shared coprocess fd when a keystroke lands mid-cycle. The wire
# path must skip it and keep reading for its own STYLED frame, the way ghost's
# identical loop already does, instead of tearing down a healthy helper.
_MBX_TEST_ENGINE_STOPPED=0
_mbx_engine_write() { return 0; }
_mbx_engine_stop() { _MBX_TEST_ENGINE_STOPPED=1; }
exec {highlight_wire_fd}< <(printf '%s\n%s\n' \
    $'MBX2\t90\tACK' $'MBX2\t91\tSTYLED\t1\t4\techo')
_MBX_ENGINE_OUT_FD=$highlight_wire_fd
_mbx_highlight_refresh_wire 'echo' 4 1 "$(_mbx_deadline_after 2 && printf '%s' "$REPLY")" || \
    fail 'highlight wire path should skip a queued ACK and accept its own STYLED frame'
assert_eq 'echo' "$REPLY" 'highlight wire path should return the STYLED line after skipping an ACK'
assert_eq 4 "$_MBX_HIGHLIGHT_STYLED_POINT" \
    'highlight wire path should return the STYLED point after skipping an ACK'
assert_eq 0 "$_MBX_TEST_ENGINE_STOPPED" \
    'a queued ACK must not stop the coprocess on the highlight wire path'
exec {highlight_wire_fd}<&-

# H-7: an unrelated frame that is not an ACK is still a hard desync and must
# stop the engine, so the fix above does not widen into "ignore everything".
exec {highlight_wire_fd}< <(printf '%s\n' $'MBX2\t92\tPONG')
_MBX_ENGINE_OUT_FD=$highlight_wire_fd
_MBX_TEST_ENGINE_STOPPED=0
_mbx_highlight_refresh_wire 'echo' 4 1 "$(_mbx_deadline_after 2 && printf '%s' "$REPLY")" && \
    fail 'highlight wire path should fail on an unexpected non-ACK frame'
assert_eq 1 "$_MBX_TEST_ENGINE_STOPPED" \
    'an unexpected non-ACK frame must stop the coprocess on the highlight wire path'
exec {highlight_wire_fd}<&-
unset -f _mbx_engine_write _mbx_engine_stop
unset _MBX_ENGINE_OUT_FD _MBX_TEST_ENGINE_STOPPED

unset MBX_BIN MBX_HIGHLIGHT _MBX_HIGHLIGHT_PLAIN _MBX_HIGHLIGHT_POINT \
    _MBX_HIGHLIGHT_ACTIVE _MBX_HIGHLIGHT_INSTALLED _MBX_HIGHLIGHT_BOUND READLINE_LINE READLINE_POINT
rm -rf "$highlight_stub_dir"

source "$ROOT/bash/editor.bash"
READLINE_LINE='keep-me'
READLINE_POINT=4
MBX_EDITOR_INSERT_TOKEN=$'printf \033hijack'
_mbx_editor_insert_token
assert_eq 'keep-me' "$READLINE_LINE" \
    'editor must not insert a token that contains C0'
assert_eq 4 "$READLINE_POINT" \
    'editor must leave the cursor unchanged when the token contains C0'
MBX_EDITOR_INSERT_TOKEN='hello'
_mbx_editor_insert_token
assert_eq 'keephello-me' "$READLINE_LINE" \
    'editor should insert a printable token at the cursor'
assert_eq 9 "$READLINE_POINT" 'editor should advance the cursor by the token length'
unset MBX_EDITOR_INSERT_TOKEN READLINE_LINE READLINE_POINT

printf 'PASS: focused Bash module contracts\n'
