#[path = "common/mod.rs"]
mod common;

use common::*;
use mbx_pty::{visible_contains, visible_text};
use std::time::Duration;

const RIGHT: &[u8] = b"\x1b[C";

fn send_keys(session: &mut mbx_pty::PtySession, keys: &[u8]) {
    session.write_all(keys, deadline(2)).expect("keys");
}

fn ghost_env<'a>(
    data_home: &'a str,
    histfile: &'a str,
    extra: &'a [(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut env = enabled_env(data_home, histfile, extra);
    env.push(("MBX_GHOST", "1"));
    env
}

fn record_echo(session: &mut mbx_pty::PtySession, marker: &str) {
    type_line(session, &format!("echo MBX_GHST:{marker}"));
    wait_all(session, &[&format!("\nMBX_GHST:{marker}"), "> "]);
}

fn assert_no_output(session: &mut mbx_pty::PtySession, marker: &str) {
    let result = session.read_until(
        std::time::Instant::now() + Duration::from_secs(1),
        mbx_pty::DEFAULT_CAPTURE_LIMIT,
        |output| visible_contains(output, marker),
    );
    assert!(
        matches!(result, Err(mbx_pty::PtyError::Timeout(_))),
        "ghost executed before Enter: {:?}",
        result.map(|output| visible_text(&output))
    );
}

#[test]
fn typing_shows_suffix_and_enter_runs_typed_prefix() {
    let home = TempHome::new("ghst-g1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &ghost_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_echo(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("echo MBX_GHST:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_GHST:alpha"]);
    assert_no_output(&mut session, "\nMBX_GHST:alpha");
    session.write_str("\n", deadline(2)).expect("enter");
    let output = wait_all(&mut session, &["MBX_GHST:a\n", "> "]);
    assert!(
        !visible_contains(&output, "MBX_GHST:alpha\n"),
        "unaccepted ghost executed: {:?}",
        visible_text(&output)
    );
    exit_and_wait(&mut session);
}

#[test]
fn right_arrow_accepts_full_suggestion() {
    let home = TempHome::new("ghst-g2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &ghost_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_echo(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("echo MBX_GHST:a", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_GHST:alpha"]);
    send_keys(&mut session, RIGHT);
    assert_no_output(&mut session, "\nMBX_GHST:alpha");
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["MBX_GHST:alpha\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn ghost_off_does_not_extend_the_line() {
    let home = TempHome::new("ghst-g3");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_echo(&mut session, "alpha");
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("echo MBX_GHST:a", deadline(2))
        .expect("type prefix");
    assert_no_output(&mut session, "echo MBX_GHST:alpha");
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["MBX_GHST:a\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn missing_helper_still_inserts_typed_bytes() {
    let home = TempHome::new("ghst-g4");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &ghost_env(
            data_home_s,
            histfile_s,
            &[("MBX_BIN", "/nonexistent/mbx-ghost-helper")],
        ),
    );
    wait_for(&mut session, "> ");
    session
        .write_str("echo MBX_GHST:typed", deadline(2))
        .expect("type");
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["MBX_GHST:typed\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn default_install_sets_bound_flag() {
    let home = TempHome::new("ghst-g5");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &ghost_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    session
        .write_str(
            "[[ ${_MBX_GHOST_BOUND:-missing} == 1 ]] && printf 'MBX_GHST:bound\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_GHST:bound", "> "]);
    exit_and_wait(&mut session);
}
