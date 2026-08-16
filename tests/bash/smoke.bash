#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/../.." && pwd -P)
MBX_TEST_BIN=${1:-"$ROOT/target/debug/mbx"}

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

[[ -x $MBX_TEST_BIN ]] || fail "mbx binary is missing: $MBX_TEST_BIN"

noninteractive=$(bash --noprofile --norc -c '
    before=${PROMPT_COMMAND-unset}
    source "$1/bash/init.bash"
    printf "%s:%s" "${_MBX_INITIALIZED-unset}" "${PROMPT_COMMAND-unset}"
' _ "$ROOT")
[[ $noninteractive == 'unset:unset' ]] || fail 'non-interactive Bash was modified'

transcript_dir=$(mktemp -d "${TMPDIR:-/tmp}/mbx-tests.XXXXXXXX")
baseline_log=$transcript_dir/baseline.log
enhanced_log=$transcript_dir/enhanced.log
baseline_markers=$transcript_dir/baseline.markers
enhanced_markers=$transcript_dir/enhanced.markers
trace_err=$transcript_dir/trace.err
trace_out=$transcript_dir/trace.out
cleanup() {
    local file
    for file in "$baseline_log" "$enhanced_log" "$baseline_markers" \
        "$enhanced_markers" "$trace_err" "$trace_out"; do
        [[ ! -e $file ]] || unlink "$file"
    done
    rmdir "$transcript_dir" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

env PS1='' PS2='' TERM=dumb bash --noprofile --norc -i \
    <"$ROOT/tests/bash/corpus.bash" >"$baseline_log" 2>&1
env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" MBX_COLOR=never MBX_ICONS=never \
    TERM=dumb bash --noprofile --rcfile "$ROOT/tests/bash/interactive.rc" -i \
    <"$ROOT/tests/bash/corpus.bash" >"$enhanced_log" 2>&1

grep -o 'MBX_TEST:[^[:cntrl:]]*' "$baseline_log" >"$baseline_markers"
grep -o 'MBX_TEST:[^[:cntrl:]]*' "$enhanced_log" >"$enhanced_markers"
cmp -s "$baseline_markers" "$enhanced_markers" || {
    diff -u "$baseline_markers" "$enhanced_markers" >&2 || true
    fail 'Bash corpus semantics changed after MBX initialization'
}

hook_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" TERM=dumb bash --noprofile --norc -i 2>/dev/null <<'EOF'
trap ':' DEBUG
original=$(trap -p DEBUG)
source "$MBX_TEST_ROOT/bash/init.bash"
after=$(trap -p DEBUG)
[[ $after == "$original" ]] && printf 'preserved:%s\n' "${_MBX_DURATION_TIMING:-missing}"
exit
EOF
)
[[ $hook_state == *'preserved:0'* ]] || fail 'an existing DEBUG trap was replaced'

prompt_command_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" TERM=dumb \
    bash --noprofile --norc -i 2>/dev/null <<'EOF'
PROMPT_COMMAND='printf "ORIGINAL_STATUS:%s\n" "$?"'
source "$MBX_TEST_ROOT/bash/init.bash"
false
PROMPT_COMMAND=()
printf 'PROMPT_COMMAND_DONE\n'
exit
EOF
)
[[ $prompt_command_state == *'ORIGINAL_STATUS:1'* ]] || fail 'existing PROMPT_COMMAND did not receive the command status'

timing_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" MBX_ENABLE_DURATION_TIMING=1 \
    TERM=dumb bash --noprofile --norc -i 2>/dev/null <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
sleep 0.02
duration=${_MBX_LAST_DURATION_MS:--}
printf 'TIMING:%s:%s\n' "${_MBX_DURATION_TIMING:-missing}" "$duration"
exit
EOF
)
timing_value=${timing_state##*TIMING:1:}
timing_value=${timing_value%%[^0-9]*}
[[ $timing_value =~ ^[0-9]+$ ]] && (( timing_value >= 10 )) || fail 'opt-in duration timing did not record the command'

fallback_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN=/definitely/missing/mbx MBX_COLOR=never \
    TERM=dumb bash --noprofile --norc -i 2>/dev/null <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
false
printf 'FALLBACK:status=%s,engine=%s\n' "${_MBX_LAST_STATUS:-missing}" "${_MBX_ENGINE_READY:-missing}"
exit
EOF
)
[[ $fallback_state == *'FALLBACK:status=1,engine=0'* ]] || fail 'fallback did not preserve status'

recovery_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" TERM=dumb \
    bash --noprofile --norc -i 2>/dev/null <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
engine_pid=$_MBX_ENGINE_CHILD_PID
kill "$engine_pid"
wait "$engine_pid" 2>/dev/null || true
true
printf 'RECOVERY:alive:engine=%s\n' "${_MBX_ENGINE_READY:-missing}"
exit
EOF
)
[[ $recovery_state == *'RECOVERY:alive:engine=0'* ]] || fail 'helper exit did not degrade to the per-call renderer'

idempotent_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" TERM=dumb \
    bash --noprofile --norc -i 2>/dev/null <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
before=$(declare -p PROMPT_COMMAND)
source "$MBX_TEST_ROOT/bash/init.bash"
after=$(declare -p PROMPT_COMMAND)
[[ $before == "$after" && ${_MBX_INITIALIZED} == 1 ]] && printf 'idempotent:ok\n'
exit
EOF
)
[[ $idempotent_state == *'idempotent:ok'* ]] || fail 're-sourcing init.bash was not idempotent'

: >"$trace_err"
env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" TERM=dumb \
    bash --noprofile --norc -i >"$trace_out" 2>"$trace_err" <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
false
printf 'NO_TRACE_PROBE\n'
exit
EOF
if grep -F -q 'mbx trace' "$trace_err" "$trace_out"; then
    fail 'default install emitted helper traces'
fi
grep -F -q 'NO_TRACE_PROBE' "$trace_out" || \
    fail 'default-off trace smoke lost its probe'

printf 'PASS: Bash compatibility smoke suite\n'
