#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/../.." && pwd -P)
MBX_TEST_BIN=${1:-"$ROOT/target/debug/mbx"}

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

# Isolated HOME tests must not inherit the runner's XDG_* paths (M-057).
iso() {
    local home=$1
    shift
    env HOME="$home" XDG_CONFIG_HOME= XDG_DATA_HOME= "$@"
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
cleanup() {
    local file
    for file in "$baseline_log" "$enhanced_log" "$baseline_markers" "$enhanced_markers"; do
        [[ ! -e $file ]] || unlink "$file"
    done
    rmdir "$transcript_dir" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

env PS1='' PS2='' TERM=dumb bash --noprofile --norc -i \
    <"$ROOT/tests/bash/corpus.bash" >"$baseline_log" 2>&1
env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" MBX_COLOR=never MBX_ICONS=never \
    MBX_GHOST= MBX_HISTORY= TERM=dumb bash --noprofile --rcfile "$ROOT/tests/bash/interactive.rc" -i \
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

idempotence_state=$(env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" TERM=dumb \
    bash --noprofile --norc -i 2>/dev/null <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
first_len=${#PROMPT_COMMAND[@]}
source "$MBX_TEST_ROOT/bash/init.bash"
printf 'IDEM:%s:%s:%s\n' "${_MBX_INITIALIZED:-missing}" "$first_len" "${#PROMPT_COMMAND[@]}"
exit
EOF
)
[[ $idempotence_state == *'IDEM:1:2:2'* ]] || fail "re-sourcing init.bash was not idempotent: $idempotence_state"

[[ $(<"$ROOT/scripts/dev-setup.bash") == *'does not modify ~/.bashrc'* ]] || \
    fail 'dev-setup.bash must state that it does not modify ~/.bashrc'
if grep -E '>>[[:space:]]*.*bashrc|>[[:space:]]*.*bashrc' "$ROOT/scripts/dev-setup.bash"; then
    fail 'dev-setup.bash must not redirect into a bashrc path'
fi

install_home=$(mktemp -d "${TMPDIR:-/tmp}/mbx-install.XXXXXXXX")
install_out=$(iso "$install_home" bash "$ROOT/scripts/install.bash" \
    --profile comfort --no-build)
[[ $install_out == *"profile comfort"* ]] || \
    fail "install --no-build did not write a comfort profile: $install_out"
[[ -f $install_home/.config/mbx/config.bash ]] || \
    fail 'install --no-build must write ~/.config/mbx/config.bash'
[[ $(<"$install_home/.config/mbx/config.bash") == *MBX_HISTORY=1* ]] || \
    fail 'comfort profile must opt in to history'
[[ $(<"$install_home/.config/mbx/config.bash") != *'export MBX_HIGHLIGHT=1'* ]] || \
    fail 'comfort profile must not enable highlight'
[[ ! -e $install_home/.bashrc ]] || \
    fail 'install without --bashrc must not create ~/.bashrc'
printf 'KEEP\n' >"$install_home/.bashrc"
iso "$install_home" bash "$ROOT/scripts/install.bash" \
    --profile comfort --no-build --bashrc >/dev/null
grep -Fq '# >>> mbx begin' "$install_home/.bashrc" || \
    fail 'install --bashrc must write a managed block'
grep -Fq 'KEEP' "$install_home/.bashrc" || \
    fail 'install --bashrc must preserve existing bashrc bytes'
iso "$install_home" bash "$ROOT/scripts/install.bash" \
    --profile comfort --no-build --bashrc >/dev/null
begin_count=$(grep -c '# >>> mbx begin' "$install_home/.bashrc")
[[ $begin_count == 1 ]] || fail "install --bashrc must be idempotent, got $begin_count blocks"
iso "$install_home" bash "$ROOT/scripts/install.bash" --uninstall-bashrc >/dev/null
grep -Fq '# >>> mbx begin' "$install_home/.bashrc" && \
    fail 'uninstall-bashrc must remove the managed block'
grep -Fq 'KEEP' "$install_home/.bashrc" || \
    fail 'uninstall-bashrc must keep the rest of bashrc'
mkdir -p "$install_home/dotfiles"
printf 'LINKKEEP\n' >"$install_home/dotfiles/bashrc"
rm -f "$install_home/.bashrc"
ln -s "$install_home/dotfiles/bashrc" "$install_home/.bashrc"
iso "$install_home" bash "$ROOT/scripts/install.bash" \
    --profile comfort --no-build --bashrc >/dev/null
[[ -L $install_home/.bashrc ]] || \
    fail 'install --bashrc must keep a HOME-local bashrc symlink'
grep -Fq '# >>> mbx begin' "$install_home/dotfiles/bashrc" || \
    fail 'install --bashrc must write the symlink target'
grep -Fq 'LINKKEEP' "$install_home/dotfiles/bashrc" || \
    fail 'install --bashrc must preserve bytes in the symlink target'
iso "$install_home" bash "$ROOT/scripts/install.bash" --uninstall-bashrc >/dev/null
[[ -L $install_home/.bashrc ]] || \
    fail 'uninstall-bashrc must keep a HOME-local bashrc symlink'
grep -Fq '# >>> mbx begin' "$install_home/dotfiles/bashrc" && \
    fail 'uninstall-bashrc must strip the managed block from the symlink target'
outside_bashrc=$(mktemp "${TMPDIR:-/tmp}/mbx-outside-bashrc.XXXXXXXX")
printf 'OUTSIDE\n' >"$outside_bashrc"
ln -sf "$outside_bashrc" "$install_home/.bashrc"
if iso "$install_home" bash "$ROOT/scripts/install.bash" \
    --profile comfort --no-build --bashrc >/dev/null 2>&1; then
    fail 'install --bashrc must refuse a bashrc symlink outside HOME'
fi
[[ $(<"$outside_bashrc") == OUTSIDE ]] || \
    fail 'refused bashrc symlink must not rewrite the outside target'
rm -f "$outside_bashrc"
if iso "$install_home" bash "$ROOT/scripts/install.bash" \
    --profile nope --no-build >/dev/null 2>&1; then
    fail 'install --profile nope must fail'
fi
highlight_home=$(mktemp -d "${TMPDIR:-/tmp}/mbx-install-hl.XXXXXXXX")
iso "$highlight_home" bash "$ROOT/scripts/install.bash" \
    --profile highlight --no-build >/dev/null
[[ $(<"$highlight_home/.config/mbx/config.bash") == *'export MBX_HIGHLIGHT=1'* ]] || \
    fail 'highlight profile must opt in to highlighting'
[[ $(<"$highlight_home/.config/mbx/config.bash") != *'export MBX_GHOST=1'* ]] || \
    fail 'highlight profile must not enable ghost'
[[ ! -e $highlight_home/.bashrc ]] || \
    fail 'highlight install without --bashrc must not create ~/.bashrc'
rm -rf "$highlight_home"
rm -rf "$install_home"

if grep -Eq '^[[:space:]]*eval[[:space:]]' "$ROOT/scripts/configure.bash"; then
    fail 'configure.bash must not eval user answers'
fi
cfg_home=$(mktemp -d "${TMPDIR:-/tmp}/mbx-configure.XXXXXXXX")
cfg_answers=$(mktemp "${TMPDIR:-/tmp}/mbx-answers.XXXXXXXX")
cat >"$cfg_answers" <<'EOF'
preset=comfort
highlight=1
wrap=git;rm
exclude=git *
bashrc=0
EOF
cfg_out=$(iso "$cfg_home" bash "$ROOT/scripts/configure.bash" \
    --answers "$cfg_answers" --no-build 2>&1) || \
    fail "configure --answers comfort should succeed: $cfg_out"
[[ $cfg_out == *'highlight left off'* ]] || \
    fail "ghost+highlight answers must force highlight off: $cfg_out"
[[ -f $cfg_home/.config/mbx/config.bash ]] || \
    fail 'configure --answers must write ~/.config/mbx/config.bash'
[[ $(<"$cfg_home/.config/mbx/config.bash") == *'export MBX_HISTORY=1'* ]] || \
    fail 'configure comfort answers must enable history'
[[ $(<"$cfg_home/.config/mbx/config.bash") == *'export MBX_GHOST=1'* ]] || \
    fail 'configure comfort answers must enable ghost'
[[ $(<"$cfg_home/.config/mbx/config.bash") != *'export MBX_HIGHLIGHT=1'* ]] || \
    fail 'configure must not enable highlight when ghost is on'
[[ $(<"$cfg_home/.config/mbx/config.bash") != *'export MBX_COMP_WRAP='* ]] || \
    fail 'hostile wrap tokens must not be written'
[[ $(<"$cfg_home/.config/mbx/config.bash") == *MBX_HISTORY_EXCLUDE* ]] || \
    fail 'configure should write a safe exclude glob'
[[ ! -e $cfg_home/.bashrc ]] || \
    fail 'configure without bashrc=1 must not create ~/.bashrc'
printf 'KEEP\n' >"$cfg_home/.bashrc"
cat >"$cfg_answers" <<'EOF'
preset=highlight
wrap=git
bashrc=1
EOF
iso "$cfg_home" bash "$ROOT/scripts/configure.bash" \
    --answers "$cfg_answers" --no-build >/dev/null
grep -Fq '# >>> mbx begin' "$cfg_home/.bashrc" || \
    fail 'configure bashrc=1 must write a managed block'
grep -Fq 'KEEP' "$cfg_home/.bashrc" || \
    fail 'configure bashrc=1 must preserve existing bashrc bytes'
[[ $(<"$cfg_home/.config/mbx/config.bash") == *'export MBX_HIGHLIGHT=1'* ]] || \
    fail 'highlight preset must enable highlighting'
[[ $(<"$cfg_home/.config/mbx/config.bash") != *'export MBX_GHOST=1'* ]] || \
    fail 'highlight preset must not enable ghost'
bad_answers=$(mktemp "${TMPDIR:-/tmp}/mbx-answers-bad.XXXXXXXX")
printf 'not_a_key=1\n' >"$bad_answers"
if iso "$cfg_home" bash "$ROOT/scripts/configure.bash" \
    --answers "$bad_answers" --no-build >/dev/null 2>&1; then
    fail 'configure must reject unknown answers keys'
fi
printf 'exclude=$(reboot)\n' >"$bad_answers"
if iso "$cfg_home" bash "$ROOT/scripts/configure.bash" \
    --answers "$bad_answers" --no-build >/dev/null 2>&1; then
    fail 'configure must reject exclude values with $'
fi
menu_home=$(mktemp -d "${TMPDIR:-/tmp}/mbx-configure-menu.XXXXXXXX")
menu_out=$(printf '1\nw\n' | iso "$menu_home" bash "$ROOT/scripts/configure.bash" \
    --no-build 2>&1) || fail "configure menu should accept piped choices: $menu_out"
[[ -f $menu_home/.config/mbx/config.bash ]] || \
    fail 'piped configure menu must write a config file'
[[ $(<"$menu_home/.config/mbx/config.bash") == *'export MBX_GHOST=1'* ]] || \
    fail 'piped comfort choice must enable ghost'
[[ ! -e $menu_home/.bashrc ]] || \
    fail 'piped configure menu must not write bashrc by default'
rm -f "$cfg_answers" "$bad_answers"
rm -rf "$cfg_home" "$menu_home"

bashrc_home=$(mktemp -d "${TMPDIR:-/tmp}/mbx-bashrc.XXXXXXXX")
printf 'SENTINEL\n' >"$bashrc_home/.bashrc"
bashrc_state=$(iso "$bashrc_home" env MBX_TEST_ROOT="$ROOT" MBX_BIN="$MBX_TEST_BIN" \
    TERM=dumb bash --noprofile --norc -i 2>/dev/null <<'EOF'
source "$MBX_TEST_ROOT/bash/init.bash"
printf 'BASHRC_SOURCED:%s\n' "${_MBX_INITIALIZED:-missing}"
exit
EOF
)
[[ $bashrc_state == *'BASHRC_SOURCED:1'* ]] || \
    fail "isolated HOME init did not complete: $bashrc_state"
[[ $(<"$bashrc_home/.bashrc") == SENTINEL ]] || \
    fail 'source init.bash modified ~/.bashrc'
rm -f "$bashrc_home/.bashrc" "$bashrc_home/.bash_history"
rmdir "$bashrc_home" 2>/dev/null || true

printf 'PASS: Bash compatibility smoke suite\n'
