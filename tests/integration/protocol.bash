#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "${BASH_SOURCE[0]%/*}/../.." && pwd -P)
MBX_TEST_BIN=${1:-"$ROOT/target/debug/mbx"}

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

handshake=$("$MBX_TEST_BIN" handshake)
[[ $handshake == 'mbx/0.1.0 ready' ]] || fail 'CLI handshake failed'

deleted_cwd=$(mktemp -d "${TMPDIR:-/tmp}/mbx-deleted-cwd.XXXXXXXX")
version_without_cwd=$(
    cd "$deleted_cwd"
    rmdir "$deleted_cwd"
    "$MBX_TEST_BIN" --version
)
[[ $version_without_cwd == 'mbx 0.1.0' ]] || \
    fail 'a non-prompt command unexpectedly required a working directory'

ping=$(printf 'MBX1\t17\tPING\n' | "$MBX_TEST_BIN" serve --stdio)
[[ $ping == $'MBX1\t17\tPONG' ]] || fail 'stdio PING/PONG failed'

prompt=$(printf 'MBX1\t18\tPROMPT\t/tmp/project\t127\t2500\t35\n' | \
    "$MBX_TEST_BIN" serve --stdio)
[[ $prompt == *$'MBX1\t18\tPROMPT\t'* ]] || fail 'stdio prompt response failed'
[[ $prompt == *'exit 127'* && $prompt == *'2.5s'* ]] || fail 'prompt context was not rendered'

malformed=$(printf 'MBX1\tbogus\tPING\n' | "$MBX_TEST_BIN" serve --stdio)
[[ $malformed == $'MBX1\t0\tERROR\tinvalid request id' ]] || fail 'malformed input did not fail closed'

plain=$(TERM=dumb "$MBX_TEST_BIN" prompt --cwd /tmp/project --status 0 --ascii --disable-git)
[[ $plain == '/tmp/project\n> ' ]] || fail 'plain prompt fallback changed'

piped=$("$MBX_TEST_BIN" prompt --cwd /tmp/project --status 0 --ascii --disable-git | cat)
[[ $piped == '/tmp/project\n> ' ]] || fail 'piped direct prompt must be plain text'
[[ $piped != *'\[\e['* ]] || fail 'piped direct prompt must not emit CSI color'

flagged=$("$MBX_TEST_BIN" prompt --cwd /tmp/project --flags 34 | cat)
[[ $flagged == *'\[\e['* ]] || fail 'explicit --flags must preserve color under a pipe'

printf 'PASS: helper protocol integration\n'
