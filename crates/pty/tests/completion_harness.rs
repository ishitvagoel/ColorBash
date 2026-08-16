mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, SpawnOptions, WinSize};
use std::fs;
use std::path::Path;

const TAB: u8 = 0x09;

fn spawn_fixture_shell(home: &Path) -> PtySession {
    spawn_mbx_shell(home, &[("MBX_COMP_FIXTURES", "1")], "")
}

fn spawn_mbx_shell(home: &Path, extra_env: &[(&str, &str)], rc_prelude: &str) -> PtySession {
    fs::write(
        home.join("rc.bash"),
        format!("{rc_prelude}source \"${{MBX_TEST_ROOT}}/bash/init.bash\"\n"),
    )
    .expect("rcfile");
    let mut options = SpawnOptions::new("/bin/bash")
        .arg("--noprofile")
        .arg("--rcfile")
        .arg(home.join("rc.bash"))
        .arg("-i")
        .clear_env()
        .env("PATH", path_env())
        .env("HOME", home)
        .env("TERM", "xterm")
        .env("USER", "mbx")
        .env("HISTFILE", "/dev/null")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("MBX_TEST_ROOT", workspace_root())
        .env("MBX_BIN", mbx_bin())
        .env("MBX_COLOR", "never")
        .env("MBX_ICONS", "never")
        .env("MBX_DISABLE_GIT", "1")
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 });
    for &(key, value) in extra_env {
        options = options.env(key, value);
    }
    PtySession::spawn(options).expect("completion shell spawn")
}

fn wait_prompt(session: &mut PtySession) {
    wait_all(session, &["> "]);
}

fn send_tab(session: &mut PtySession) {
    session.write_all(&[TAB], deadline(2)).expect("tab");
}

#[test]
fn probe_snapshot_captures_comp_state() {
    let home = TempHome::new("comp-h2");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_probe mbx_co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session
        .write_str("\n", deadline(2))
        .expect("submit probe line");
    wait_prompt(&mut session);
    session
        .write_str(
            "printf 'MBX_COMP:%s:%s:%s:%s\\n' \"${_MBX_COMP_LINE:-}\" \"${_MBX_COMP_POINT:-}\" \"${_MBX_COMP_CWORD:-}\" \"${_MBX_COMP_LAST_REPLY:-}\"\n",
            deadline(2),
        )
        .expect("dump snapshot");
    wait_all(
        &mut session,
        &[
            "\nMBX_COMP:mbx_comp_probe mbx_co:21:1:mbx_comp_candidate",
            "> ",
        ],
    );
}

#[test]
fn stock_ls_completion_is_not_wrapped() {
    let home = TempHome::new("comp-h3");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str(
            "complete -p ls 2>/dev/null | grep -F '_mbx_comp' >/dev/null && \
printf 'MBX_COMP:wrapped\\n' || printf 'MBX_COMP:unwrapped\\n'\n",
            deadline(2),
        )
        .expect("query ls completion");
    wait_all(&mut session, &["\nMBX_COMP:unwrapped", "> "]);
    session
        .write_str(
            "_mbx_comp_command_uses_adapter mbx_comp_probe && \
printf 'MBX_COMP:probe_wrapped\\n' || printf 'MBX_COMP:probe_missing\\n'\n",
            deadline(2),
        )
        .expect("query probe wrap");
    wait_all(&mut session, &["\nMBX_COMP:probe_wrapped", "> "]);
}

#[test]
fn unique_filename_tab_completes_like_stock() {
    let home = TempHome::new("comp-h4");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("ls MBX_COMP_U", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_COMP_UNIQUE", "> "]);
}

#[test]
fn unique_file_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-p1");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s|\\n' MBX_COMP_U", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_UNIQUE|", "> "]);
}

#[test]
fn spaced_filename_completion_preserves_stock_quoting() {
    let home = TempHome::new("comp-p2");
    fs::write(home.path().join("MBX_COMP_A B"), "probe\n").expect("spaced file");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s|\\n' MBX_COMP_A", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_A B|", "> "]);
}

#[test]
fn wrapped_flag_nospace_concatenates_suffix() {
    let home = TempHome::new("comp-p3");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_flag_nospace --mbx-co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session
        .write_str("X\n", deadline(2))
        .expect("suffix and submit");
    wait_all(&mut session, &["\nGOT:--mbx-comp-flagX|", "> "]);
}

#[test]
fn wrapped_flag_default_suffix_separates_next_word() {
    let home = TempHome::new("comp-p4");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_flag --mbx-co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session
        .write_str("X\n", deadline(2))
        .expect("suffix and submit");
    wait_all(&mut session, &["\nGOT:--mbx-comp-flag X|", "> "]);
}

#[test]
fn stock_printf_completion_is_not_wrapped() {
    let home = TempHome::new("comp-wrap-check");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str(
            "complete -p printf 2>/dev/null | grep -F '_mbx_comp' >/dev/null && \
printf 'MBX_COMP:wrapped\\n' || printf 'MBX_COMP:unwrapped\\n'\n",
            deadline(2),
        )
        .expect("query printf completion");
    wait_all(&mut session, &["\nMBX_COMP:unwrapped", "> "]);
    session
        .write_str(
            "_mbx_comp_command_uses_flag_adapter mbx_comp_flag && \
printf 'MBX_COMP:flag_wrapped\\n' || printf 'MBX_COMP:flag_missing\\n'\n",
            deadline(2),
        )
        .expect("query flag wrap");
    wait_all(&mut session, &["\nMBX_COMP:flag_wrapped", "> "]);
}

#[test]
fn default_install_does_not_define_fixtures() {
    let home = TempHome::new("comp-f1");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str(
            "if declare -F mbx_comp_flag >/dev/null 2>&1 || \
complete -p mbx_comp_flag >/dev/null 2>&1; then \
printf 'MBX_COMP:fixture_present\\n'; else printf 'MBX_COMP:fixture_absent\\n'; fi\n",
            deadline(2),
        )
        .expect("query default fixtures");
    wait_all(&mut session, &["\nMBX_COMP:fixture_absent", "> "]);
}

#[test]
fn alias_file_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-l1");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("alias mbxpr=printf\n", deadline(2))
        .expect("define alias");
    wait_prompt(&mut session);
    session
        .write_str("mbxpr 'GOT:%s|\\n' MBX_COMP_U", deadline(2))
        .expect("type alias prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_UNIQUE|", "> "]);
}

#[test]
fn redirection_target_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-l2");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'x' > MBX_COMP_U", deadline(2))
        .expect("type redirect prefix");
    send_tab(&mut session);
    session
        .write_str("\n", deadline(2))
        .expect("create via redirect");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s|\\n' MBX_COMP_*\n", deadline(2))
        .expect("glob completed file");
    wait_all(&mut session, &["\nGOT:MBX_COMP_UNIQUE|", "> "]);
}

#[test]
fn unicode_filename_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-l3");
    fs::write(home.path().join("MBX_COMP_café"), "probe\n").expect("unicode file");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s|\\n' MBX_COMP_c", deadline(2))
        .expect("type unicode prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_café|", "> "]);
}

#[test]
fn incomplete_quote_file_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-l4");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s|\\n' 'MBX_COMP_U", deadline(2))
        .expect("type incomplete quote prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_UNIQUE|", "> "]);
}

#[test]
fn double_dash_file_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-n1");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s|\\n' -- MBX_COMP_U", deadline(2))
        .expect("type double-dash prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_UNIQUE|", "> "]);
}

#[test]
fn nested_substitution_file_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-n2");
    fs::write(home.path().join("MBX_COMP_UNIQUE"), "probe\n").expect("unique file");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    // Plan's `: $(printf ...)` captures printf stdout in the substitution, so GOT
    // never reaches the terminal. echo $(...) preserves nested file completion and
    // prints the measured stock line (recorded in comp-002-dash-nested-plan.md).
    session
        .write_str("echo $(printf 'GOT:%s|\\n' MBX_COMP_U", deadline(2))
        .expect("type nested prefix");
    send_tab(&mut session);
    session
        .write_str(")\n", deadline(2))
        .expect("close substitution and submit");
    wait_all(&mut session, &["\nGOT:MBX_COMP_UNIQUE|", "> "]);
}

#[test]
fn unsupported_wordlist_completion_skips_wrap() {
    let home = TempHome::new("comp-s1");
    let prelude = "\
mbx_comp_words() { printf 'GOT:%s|\\n' \"$*\"; }
complete -W 'mbx_word_alpha' mbx_comp_words
";
    let mut session = spawn_mbx_shell(home.path(), &[], prelude);
    wait_prompt(&mut session);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_words\n", deadline(2))
        .expect("attempt wrap on -W spec");
    wait_prompt(&mut session);
    session
        .write_str(
            "complete -p mbx_comp_words 2>/dev/null | grep -Fq '_mbx_comp_existing_adapter' && \
printf 'MBX_COMP:wrapped\\n' || printf 'MBX_COMP:unwrapped\\n'\n",
            deadline(2),
        )
        .expect("query wrap state");
    wait_all(&mut session, &["\nMBX_COMP:unwrapped", "> "]);
    session
        .write_str(
            "complete -p mbx_comp_words 2>/dev/null | grep -Fq ' -W ' && \
printf 'MBX_COMP:has_w\\n' || printf 'MBX_COMP:no_w\\n'\n",
            deadline(2),
        )
        .expect("query -W spec");
    wait_all(&mut session, &["\nMBX_COMP:has_w", "> "]);
    session
        .write_str("mbx_comp_words mbx_w", deadline(2))
        .expect("type wordlist prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:mbx_word_alpha|", "> "]);
}

#[test]
fn slow_wrapped_function_completion_preserves_stock_bytes() {
    let home = TempHome::new("comp-s2");
    let prelude = "\
mbx_comp_slow() { printf 'GOT:%s|\\n' \"$*\"; }
_mbx_comp_slow_backend() {
    sleep 0.2
    COMPREPLY=(--mbx-comp-slow)
}
complete -F _mbx_comp_slow_backend mbx_comp_slow
";
    let mut session = spawn_mbx_shell(home.path(), &[], prelude);
    wait_prompt(&mut session);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_slow\n", deadline(2))
        .expect("wrap slow backend");
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_slow --mbx-sl", deadline(2))
        .expect("type slow prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:--mbx-comp-slow|", "> "]);
}

#[test]
fn stateful_wrapped_function_reads_live_shell_state() {
    let home = TempHome::new("comp-s3");
    let prelude = "\
mbx_comp_state() { printf 'GOT:%s|\\n' \"$*\"; }
_mbx_comp_state_backend() {
    COMPREPLY=(\"${MBX_COMP_STATE_TOKEN:-missing}\")
}
complete -F _mbx_comp_state_backend mbx_comp_state
";
    let mut session = spawn_mbx_shell(home.path(), &[], prelude);
    wait_prompt(&mut session);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_state\n", deadline(2))
        .expect("wrap state backend");
    wait_prompt(&mut session);
    session
        .write_str("MBX_COMP_STATE_TOKEN=live-alpha\n", deadline(2))
        .expect("set live state token");
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_state liv", deadline(2))
        .expect("type state prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:live-alpha|", "> "]);
}

#[test]
fn empty_compreply_wrapped_function_inserts_nothing() {
    let home = TempHome::new("comp-s4");
    let prelude = "\
mbx_comp_empty() { printf 'GOT:%s|\\n' \"$*\"; }
_mbx_comp_empty_backend() {
    COMPREPLY=()
}
complete -F _mbx_comp_empty_backend mbx_comp_empty
";
    let mut session = spawn_mbx_shell(home.path(), &[], prelude);
    wait_prompt(&mut session);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_empty\n", deadline(2))
        .expect("wrap empty backend");
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_empty nosuch", deadline(2))
        .expect("type empty prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:nosuch|", "> "]);
}

#[test]
fn metadata_preserves_flag_insertion_bytes() {
    let home = TempHome::new("comp-k1");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_flag --mbx-co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session
        .write_str("X\n", deadline(2))
        .expect("suffix and submit");
    wait_all(&mut session, &["\nGOT:--mbx-comp-flag X|", "> "]);
}

#[test]
fn metadata_description_never_inserted() {
    let home = TempHome::new("comp-k3");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_flag --mbx-co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    let output = wait_all(&mut session, &["\nGOT:", "> "]);
    let text = String::from_utf8_lossy(&output);
    assert!(
        !text.contains("EXTRA"),
        "completion description leaked into terminal output: {text}"
    );
    assert!(
        text.contains("GOT:--mbx-comp-flag"),
        "expected stock flag insertion bytes, got: {text}"
    );
}

#[test]
fn ranking_preserves_flag_insertion_bytes() {
    let home = TempHome::new("comp-r1");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_flag --mbx-co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session
        .write_str("X\n", deadline(2))
        .expect("suffix and submit");
    wait_all(&mut session, &["\nGOT:--mbx-comp-flag X|", "> "]);
}

#[test]
fn ranking_description_never_inserted() {
    let home = TempHome::new("comp-r4");
    let mut session = spawn_fixture_shell(home.path());
    wait_prompt(&mut session);
    session
        .write_str("mbx_comp_flag --mbx-co", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    let output = wait_all(&mut session, &["\nGOT:", "> "]);
    let text = String::from_utf8_lossy(&output);
    assert!(
        !text.contains("EXTRA"),
        "completion description leaked into terminal output: {text}"
    );
    assert!(
        text.contains("GOT:--mbx-comp-flag"),
        "expected stock flag insertion bytes, got: {text}"
    );
}
