# Marker prefix lives in a variable so the literal string the smoke suite
# greps for never appears in this file's own source text. `bash -i` reading a
# script from a file echoes input lines into the same stream as program
# output, and `grep -o 'MBX_TEST:...'` cannot tell the two apart — so with the
# prefix written literally, the marker set silently included echoed source.
# That made the comparison assert something it does not name and cannot
# require: that MBX leaves Bash's input echo byte-identical, when MBX
# legitimately changes PS1/PS2 and Readline state. It also made the result
# depend on the Readline build (identical echo on a vanilla 5.0/5.2, divergent
# on Ubuntu 20.04's 5.0). With the prefix interpolated, only real program
# output can match, which is the semantic property this suite is for.
M='MBX_TEST'
printf '%s:echo=%s\n' "$M" hello
printf '%s:variable=%s\n' "$M" "$HOME"
false || printf '%s:or=fallback\n' "$M"
printf '%s\n' a b c | grep b | sed "s/^/$M:pipeline=/"
for x in a b; do printf '%s:loop=%s\n' "$M" "$x"; done
foo() { printf '%s:function=foo\n' "$M"; }
foo
alias mbx_test_alias='printf "%s:alias=ok\n" "$M"'
mbx_test_alias
sleep 0.01 &
job_pid=$!
wait "$job_pid"
printf '%s:background=ok\n' "$M"
(cd /tmp && printf '%s:subshell=%s\n' "$M" "$PWD")
read -r here_value <<< "hello"
printf '%s:here-string=%s\n' "$M" "$here_value"
read -r process_value < <(printf test)
printf '%s:process-substitution=%s\n' "$M" "$process_value"
values=(alpha beta)
printf '%s:array=%s,%s\n' "$M" "${values[0]}" "${values[1]}"
false
printf '%s:status=%s\n' "$M" "$?"
set -u
printf '%s:nounset=ok\n' "$M"
set +u
exit
