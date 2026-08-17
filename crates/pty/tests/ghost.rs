#[path = "common/mod.rs"]
mod common;

use common::*;
use mbx_pty::{visible_contains, visible_text};
use std::time::Duration;

const RIGHT: &[u8] = b"\x1b[C";
const META_F: &[u8] = b"\x1bf";
const CTRL_X_CTRL_N: &[u8] = b"\x18\x0e";

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
    wait_all(session, &[&format!("MBX_GHST:{marker}\n"), "> "]);
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
    assert_no_output(&mut session, "MBX_GHST:alpha\n");
    session.write_str("\n", deadline(2)).expect("enter");
    let output = wait_all(&mut session, &["MBX_GHST:a\n", "> "]);
    assert!(
        !visible_contains(&output, "MBX_GHST:alpha\n"),
        "unaccepted ghost executed: {:?}",
        visible_text(&output)
    );
    wait_for_count(&mbx_bin(), &data_home, 2);
    let recent = sidecar_commands(&mbx_bin(), &data_home);
    assert!(
        recent.iter().any(|command| command == "echo MBX_GHST:a"),
        "typed prefix was not admitted through accept-line: {recent:?}"
    );
    assert_eq!(
        recent
            .iter()
            .filter(|command| *command == "echo MBX_GHST:alpha")
            .count(),
        1,
        "unaccepted suffix was admitted: {recent:?}"
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
    assert_no_output(&mut session, "MBX_GHST:alpha\n");
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["MBX_GHST:alpha\n", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    let recent = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(
        recent
            .iter()
            .filter(|command| *command == "echo MBX_GHST:alpha")
            .count(),
        2,
        "accepted ghost was not admitted through accept-line: {recent:?}"
    );
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
            "[[ ${_MBX_GHOST_BOUND:-missing} == 1 && ${_MBX_GHOST_CYCLE_BOUND:-missing} == 1 ]] && printf 'MBX_GHST:bound\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["MBX_GHST:bound\n", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn alt_f_accepts_one_word() {
    let home = TempHome::new("ghst-w3");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &ghost_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo MBX_GHST:one two");
    wait_all(&mut session, &["MBX_GHST:one two\n", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);

    session
        .write_str("echo MBX_GHST:o", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_GHST:one two"]);
    send_keys(&mut session, META_F);
    assert_no_output(&mut session, "MBX_GHST:one two\n");
    session.write_str("\n", deadline(2)).expect("enter");
    let output = wait_all(&mut session, &["MBX_GHST:one\n", "> "]);
    assert!(
        !visible_contains(&output, "MBX_GHST:one two\n"),
        "unaccepted words executed: {:?}",
        visible_text(&output)
    );
    wait_for_count(&mbx_bin(), &data_home, 2);
    let recent = sidecar_commands(&mbx_bin(), &data_home);
    assert!(
        recent.iter().any(|command| command == "echo MBX_GHST:one"),
        "word-accepted prefix was not admitted: {recent:?}"
    );
    exit_and_wait(&mut session);
}

#[test]
fn ctrl_x_ctrl_n_cycles_to_older_prefix_match() {
    let home = TempHome::new("ghst-c2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &ghost_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    record_echo(&mut session, "one");
    record_echo(&mut session, "two");
    wait_for_count(&mbx_bin(), &data_home, 2);

    session
        .write_str("echo MBX_GHST:", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["echo MBX_GHST:two"]);
    send_keys(&mut session, CTRL_X_CTRL_N);
    wait_all(&mut session, &["echo MBX_GHST:one"]);
    assert_no_output(&mut session, "MBX_GHST:one\n");
    send_keys(&mut session, RIGHT);
    session.write_str("\n", deadline(2)).expect("enter");
    let output = wait_all(&mut session, &["MBX_GHST:one\n", "> "]);
    assert!(
        !visible_contains(&output, "MBX_GHST:two\n"),
        "newest match executed after cycling to the older one: {:?}",
        visible_text(&output)
    );
    wait_for_count(&mbx_bin(), &data_home, 3);
    let recent = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(
        recent
            .iter()
            .filter(|command| *command == "echo MBX_GHST:one")
            .count(),
        2,
        "cycled older match was not admitted: {recent:?}"
    );
    assert_eq!(
        recent
            .iter()
            .filter(|command| *command == "echo MBX_GHST:two")
            .count(),
        1,
        "newest match was admitted after cycling away: {recent:?}"
    );
    exit_and_wait(&mut session);
}
