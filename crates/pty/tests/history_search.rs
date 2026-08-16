#[path = "common/mod.rs"]
mod common;

use common::*;
use mbx_pty::visible_contains;
use std::time::Duration;

const CTRL_X: u8 = 0x18;
const DEFAULT_KEYSEQ: &[u8] = &[CTRL_X, b'h'];

fn send_keyseq(session: &mut mbx_pty::PtySession, keyseq: &[u8]) {
    session
        .write_all(keyseq, deadline(2))
        .expect("search key sequence");
}

fn assert_no_marker(session: &mut mbx_pty::PtySession, marker: &str) {
    let result = session.read_until(
        std::time::Instant::now() + Duration::from_secs(1),
        mbx_pty::DEFAULT_CAPTURE_LIMIT,
        |output| visible_contains(output, marker),
    );
    assert!(
        matches!(result, Err(mbx_pty::PtyError::Timeout(_))),
        "search insert executed before Enter: {:?}",
        result.map(|output| mbx_pty::visible_text(&output))
    );
}

fn record_printf(session: &mut mbx_pty::PtySession, marker: &str) {
    type_line(session, &format!("printf 'MBX_SRCH:{marker}\\n'"));
    wait_all(session, &[&format!("\nMBX_SRCH:{marker}"), "> "]);
}

#[test]
fn prefix_replace_does_not_execute_until_enter() {
    let home = TempHome::new("srch-s1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    record_printf(&mut session, "beta");
    wait_for_count(&mbx_bin(), &data_home, 2);

    session
        .write_str("printf 'MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["printf 'MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:alpha", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn empty_line_inserts_most_recent() {
    let home = TempHome::new("srch-s2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    record_printf(&mut session, "beta");
    wait_for_count(&mbx_bin(), &data_home, 2);

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:beta");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:beta", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn occupied_chord_is_not_overwritten() {
    let home = TempHome::new("srch-s3");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let prelude = concat!(
        "_mbx_user_search() { :; }\n",
        "bind -x '\"\\C-xh\": _mbx_user_search'\n",
    );
    let mut session = spawn_history_shell_rc(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[]),
        prelude,
    );
    wait_for(&mut session, "> ");
    session
        .write_str("bind -X | grep -F '_mbx_user_search'\n", deadline(2))
        .expect("query binding");
    wait_all(&mut session, &["_mbx_user_search", "> "]);
    session
        .write_str(
            "[[ ${_MBX_SEARCH_BOUND:-missing} == 0 ]] && printf 'MBX_SRCH:refused\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_SRCH:refused", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn default_chord_installs_on_stock_emacs() {
    let home = TempHome::new("srch-s7");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    session
        .write_str(
            "[[ ${_MBX_SEARCH_BOUND:-missing} == 1 ]] && printf 'MBX_SRCH:bound\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_SRCH:bound", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn history_off_leaves_typed_line() {
    let home = TempHome::new("srch-s4");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &disabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    session
        .write_str("printf 'MBX_SRCH:off\\n'", deadline(2))
        .expect("type line");
    wait_all(&mut session, &["printf 'MBX_SRCH:off\\n'"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:off", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn substring_uses_fuzzy_fallback() {
    let home = TempHome::new("srch-s5");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:zzz-needle");
    wait_all(&mut session, &["\nMBX_SRCH:zzz-needle", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("needle", deadline(2))
        .expect("type needle");
    wait_all(&mut session, &["needle"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:zzz-needle");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:zzz-needle", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn missing_helper_is_a_noop() {
    let home = TempHome::new("srch-s6");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(
            data_home_s,
            histfile_s,
            &[("MBX_BIN", "/nonexistent/mbx-search-helper")],
        ),
    );
    wait_for(&mut session, "> ");
    session
        .write_str("printf 'MBX_SRCH:missing\\n'", deadline(2))
        .expect("type line");
    wait_all(&mut session, &["printf 'MBX_SRCH:missing\\n'"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:missing", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn prefix_cycle_inserts_next_match() {
    let home = TempHome::new("srch-v1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha1");
    record_printf(&mut session, "alpha2");
    wait_for_count(&mbx_bin(), &data_home, 2);

    session
        .write_str("printf 'MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["printf 'MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha1");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:alpha1", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn empty_line_cycle_inserts_previous_recent() {
    let home = TempHome::new("srch-v2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    record_printf(&mut session, "beta");
    wait_for_count(&mbx_bin(), &data_home, 2);

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:alpha", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn snapshot_clears_at_next_prompt() {
    let home = TempHome::new("srch-v4");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:alpha", "> "]);
    session
        .write_str(
            "[[ ${#_MBX_SEARCH_MATCHES[@]} -eq 0 ]] && printf 'MBX_SRCH:cleared\\n'\n",
            deadline(2),
        )
        .expect("query snapshot");
    wait_all(&mut session, &["\nMBX_SRCH:cleared", "> "]);
    exit_and_wait(&mut session);
}
