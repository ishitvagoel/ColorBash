printf 'MBX_TEST:echo=%s\n' hello
printf 'MBX_TEST:variable=%s\n' "$HOME"
false || printf 'MBX_TEST:or=fallback\n'
printf '%s\n' a b c | grep b | sed 's/^/MBX_TEST:pipeline=/'
for x in a b; do printf 'MBX_TEST:loop=%s\n' "$x"; done
foo() { printf 'MBX_TEST:function=foo\n'; }
foo
alias mbx_test_alias='printf "MBX_TEST:alias=ok\\n"'
mbx_test_alias
sleep 0.01 &
job_pid=$!
wait "$job_pid"
printf 'MBX_TEST:background=ok\n'
(cd /tmp && printf 'MBX_TEST:subshell=%s\n' "$PWD")
read -r here_value <<< "hello"
printf 'MBX_TEST:here-string=%s\n' "$here_value"
read -r process_value < <(printf test)
printf 'MBX_TEST:process-substitution=%s\n' "$process_value"
values=(alpha beta)
printf 'MBX_TEST:array=%s,%s\n' "${values[0]}" "${values[1]}"
false
printf 'MBX_TEST:status=%s\n' "$?"
set -u
printf 'MBX_TEST:nounset=ok\n'
set +u
exit

