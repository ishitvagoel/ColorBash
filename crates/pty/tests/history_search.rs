#[path = "common/mod.rs"]
mod common;

use common::*;
use mbx_pty::{CTRL_C, CTRL_Z, WinSize, visible_contains};
use std::time::{Duration, Instant};

const CTRL_X: u8 = 0x18;
const DEFAULT_KEYSEQ: &[u8] = &[CTRL_X, b'h'];
const DEFAULT_RESTORE_KEYSEQ: &[u8] = &[CTRL_X, b'l'];

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

fn start_sleep(session: &mut mbx_pty::PtySession) {
    session
        .write_str(
            "sh -c 'printf \"MBX_PTY:running\\n\"; exec sleep 30'\n",
            deadline(2),
        )
        .expect("write");
    wait_all(session, &["\nMBX_PTY:running"]);
}

fn extract_marked(output: &[u8], prefix: &str, suffix: &str) -> String {
    let text = String::from_utf8_lossy(output);
    text.rfind(prefix)
        .and_then(|start| {
            let rest = &text[start + prefix.len()..];
            rest.find(suffix).map(|end| rest[..end].to_string())
        })
        .unwrap_or_else(|| panic!("missing {prefix}..{suffix} in {text}"))
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

#[test]
fn restore_puts_back_typed_prefix() {
    let home = TempHome::new("srch-r1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:alpha");
    wait_all(&mut session, &["\nMBX_SRCH:alpha", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("echo MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    send_keyseq(&mut session, DEFAULT_RESTORE_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_SRCH:a\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn restore_after_cycle_puts_back_typed_prefix() {
    let home = TempHome::new("srch-r1c");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:alpha1");
    wait_all(&mut session, &["\nMBX_SRCH:alpha1", "> "]);
    type_line(&mut session, "echo MBX_SRCH:alpha2");
    wait_all(&mut session, &["\nMBX_SRCH:alpha2", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);

    session
        .write_str("echo MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    send_keyseq(&mut session, DEFAULT_RESTORE_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_SRCH:a\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn restore_without_snapshot_is_a_noop() {
    let home = TempHome::new("srch-r2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    session
        .write_str("echo MBX_SRCH:keep", deadline(2))
        .expect("type line");
    wait_all(&mut session, &["echo MBX_SRCH:keep"]);
    send_keyseq(&mut session, DEFAULT_RESTORE_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:keep", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn occupied_restore_chord_is_not_overwritten() {
    let home = TempHome::new("srch-r3");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let prelude = concat!(
        "_mbx_user_search_restore() { :; }\n",
        "bind -x '\"\\C-xl\": _mbx_user_search_restore'\n",
    );
    let mut session = spawn_history_shell_rc(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[]),
        prelude,
    );
    wait_for(&mut session, "> ");
    session
        .write_str(
            "bind -X | grep -F '_mbx_user_search_restore'\n",
            deadline(2),
        )
        .expect("query binding");
    wait_all(&mut session, &["_mbx_user_search_restore", "> "]);
    session
        .write_str(
            "[[ ${_MBX_SEARCH_RESTORE_BOUND:-missing} == 0 ]] && printf 'MBX_SRCH:restore-refused\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_SRCH:restore-refused", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn default_restore_chord_installs_on_stock_emacs() {
    let home = TempHome::new("srch-r4");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    session
        .write_str(
            "[[ ${_MBX_SEARCH_RESTORE_BOUND:-missing} == 1 ]] && printf 'MBX_SRCH:restore-bound\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_SRCH:restore-bound", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn empty_line_prefers_cwd_over_newer_other_directory() {
    let home = TempHome::new("srch-c1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    std::fs::create_dir(home.path().join("dir-a")).expect("dir-a");
    std::fs::create_dir(home.path().join("dir-b")).expect("dir-b");
    let dir_a = home.path().join("dir-a");
    let dir_b = home.path().join("dir-b");
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_EXCLUDE", "cd *")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, &format!("cd '{}'", dir_a.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:dir-a");
    wait_all(&mut session, &["\nMBX_SRCH:dir-a", "> "]);
    type_line(&mut session, &format!("cd '{}'", dir_b.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:dir-b");
    wait_all(&mut session, &["\nMBX_SRCH:dir-b", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    type_line(&mut session, &format!("cd '{}'", dir_a.display()));
    wait_for(&mut session, "> ");

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:dir-b");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_SRCH:dir-a\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn empty_line_without_cwd_rows_falls_back_to_recent() {
    let home = TempHome::new("srch-c2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    std::fs::create_dir(home.path().join("dir-a")).expect("dir-a");
    std::fs::create_dir(home.path().join("dir-c")).expect("dir-c");
    let dir_a = home.path().join("dir-a");
    let dir_c = home.path().join("dir-c");
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_EXCLUDE", "cd *")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, &format!("cd '{}'", dir_a.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:dir-a");
    wait_all(&mut session, &["\nMBX_SRCH:dir-a", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    type_line(&mut session, &format!("cd '{}'", dir_c.display()));
    wait_for(&mut session, "> ");

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:dir-a");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_SRCH:dir-a\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn prefix_prefers_cwd_over_newer_other_directory() {
    let home = TempHome::new("srch-c5");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    std::fs::create_dir(home.path().join("dir-a")).expect("dir-a");
    std::fs::create_dir(home.path().join("dir-b")).expect("dir-b");
    let dir_a = home.path().join("dir-a");
    let dir_b = home.path().join("dir-b");
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_EXCLUDE", "cd *")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, &format!("cd '{}'", dir_a.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:a-home");
    wait_all(&mut session, &["\nMBX_SRCH:a-home", "> "]);
    type_line(&mut session, &format!("cd '{}'", dir_b.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:a-other");
    wait_all(&mut session, &["\nMBX_SRCH:a-other", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    type_line(&mut session, &format!("cd '{}'", dir_a.display()));
    wait_for(&mut session, "> ");

    session
        .write_str("echo MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:a-other");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_SRCH:a-home\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn prefix_without_cwd_rows_falls_back_to_global() {
    let home = TempHome::new("srch-c6");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    std::fs::create_dir(home.path().join("dir-a")).expect("dir-a");
    std::fs::create_dir(home.path().join("dir-b")).expect("dir-b");
    std::fs::create_dir(home.path().join("dir-c")).expect("dir-c");
    let dir_a = home.path().join("dir-a");
    let dir_b = home.path().join("dir-b");
    let dir_c = home.path().join("dir-c");
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_EXCLUDE", "cd *")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, &format!("cd '{}'", dir_a.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:a-home");
    wait_all(&mut session, &["\nMBX_SRCH:a-home", "> "]);
    type_line(&mut session, &format!("cd '{}'", dir_b.display()));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_SRCH:a-other");
    wait_all(&mut session, &["\nMBX_SRCH:a-other", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    type_line(&mut session, &format!("cd '{}'", dir_c.display()));
    wait_for(&mut session, "> ");

    session
        .write_str("echo MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:a-other");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_SRCH:a-other\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn ctrl_c_after_insert_leaves_a_usable_prompt() {
    let home = TempHome::new("srch-t1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("printf 'MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["printf 'MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_all(&[CTRL_C], deadline(2)).expect("cancel");
    let after_cancel = wait_for(&mut session, "> ");
    assert!(
        !visible_contains(&after_cancel, "\nMBX_SRCH:alpha"),
        "search insert executed on Ctrl+C: {:?}",
        mbx_pty::visible_text(&after_cancel)
    );
    session
        .write_str("printf 'MBX_SRCH:after_cancel\\n'\n", deadline(2))
        .expect("sentinel");
    wait_all(&mut session, &["\nMBX_SRCH:after_cancel", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn insert_restore_signal_and_resize_preserve_stty() {
    let home = TempHome::new("srch-t2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    session
        .write_str("printf 'STTY1:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("stty before");
    let first = wait_all(&mut session, &["\nSTTY1:", ":END"]);
    let before = extract_marked(&first, "STTY1:", ":END");
    record_printf(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("printf 'MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["printf 'MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    send_keyseq(&mut session, DEFAULT_RESTORE_KEYSEQ);
    session.write_all(&[CTRL_C], deadline(2)).expect("cancel");
    wait_for(&mut session, "> ");
    session
        .resize(WinSize { rows: 20, cols: 72 })
        .expect("resize");
    session
        .write_str("printf 'STTY2:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("stty after");
    let second = wait_all(&mut session, &["\nSTTY2:", ":END"]);
    let after = extract_marked(&second, "STTY2:", ":END");
    assert_eq!(before, after);
    session
        .write_str("printf 'MBX_SRCH:stty_ok\\n'\n", deadline(2))
        .expect("sentinel");
    wait_all(&mut session, &["\nMBX_SRCH:stty_ok", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn resize_after_insert_still_submits_match() {
    let home = TempHome::new("srch-t3");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("printf 'MBX_SRCH:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["printf 'MBX_SRCH:a"]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session
        .resize(WinSize { rows: 16, cols: 64 })
        .expect("resize");
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:alpha", "> "]);
    session
        .write_str("printf 'MBX_SRCH:resized\\n'\n", deadline(2))
        .expect("follow-up");
    wait_all(&mut session, &["\nMBX_SRCH:resized", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn ctrl_z_then_search_still_inserts() {
    let home = TempHome::new("srch-t4");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_EXCLUDE", "sh *")]),
    );
    wait_for(&mut session, "> ");
    record_printf(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);
    start_sleep(&mut session);
    session.write_all(&[CTRL_Z], deadline(2)).expect("ctrl-z");
    wait_for(&mut session, "> ");

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:alpha");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:alpha", "> "]);
    session
        .write_str(
            "kill -9 %1 2>/dev/null; wait 2>/dev/null || true\n",
            deadline(2),
        )
        .expect("cleanup");
    wait_for(&mut session, "> ");
    session
        .write_str("printf 'MBX_SRCH:after_stop\\n'\n", deadline(2))
        .expect("sentinel");
    wait_all(&mut session, &["\nMBX_SRCH:after_stop", "> "]);
    // First `exit` can refuse while a stopped job remains; Drop SIGKILLs the PTY.
}

fn wait_for_failed_command(bin: &std::path::Path, data_home: &std::path::Path, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let failed = query(
            bin,
            &["history", "search", "failed", "--limit", "8"],
            data_home,
        );
        if failed.lines().any(|line| line == expected) {
            return;
        }
        if Instant::now() >= deadline {
            panic!("failed search never contained {expected:?}; got {failed:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn empty_line_inserts_failed_when_opt_in() {
    let home = TempHome::new("srch-f1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_SEARCH_FAILED", "1")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "false");
    wait_for(&mut session, "> ");
    record_printf(&mut session, "ok");
    wait_for_count(&mbx_bin(), &data_home, 2);
    wait_for_failed_command(&mbx_bin(), &data_home, "false");

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:ok");
    session.write_str("\n", deadline(2)).expect("submit");
    let after_enter = wait_for(&mut session, "> ");
    assert!(
        !visible_contains(&after_enter, "\nMBX_SRCH:ok"),
        "opt-in failed empty-line insert should place `false`, not the later success: {:?}",
        mbx_pty::visible_text(&after_enter)
    );
    session
        .write_str("printf 'MBX_SRCH:failed_inserted\\n'\n", deadline(2))
        .expect("sentinel");
    wait_all(&mut session, &["\nMBX_SRCH:failed_inserted", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn empty_line_failed_falls_back_when_no_failed_rows() {
    let home = TempHome::new("srch-f2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_SEARCH_FAILED", "1")]),
    );
    wait_for(&mut session, "> ");
    record_printf(&mut session, "ok");
    wait_for_count(&mbx_bin(), &data_home, 1);

    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_marker(&mut session, "\nMBX_SRCH:ok");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_SRCH:ok", "> "]);
    session
        .write_str("printf 'MBX_SRCH:failed_fallback\\n'\n", deadline(2))
        .expect("sentinel");
    wait_all(&mut session, &["\nMBX_SRCH:failed_fallback", "> "]);
    exit_and_wait(&mut session);
}
