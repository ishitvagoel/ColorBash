mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{CTRL_C, PtySession, SpawnOptions, WinSize, visible_contains, visible_text};
use std::fs;
use std::path::Path;

const CTRL_X: u8 = 0x18;
const CTRL_Y: u8 = 0x19;
const DEFAULT_KEYSEQ: &[u8] = &[CTRL_X, CTRL_Y];
const ALT_KEYSEQ: &[u8] = &[CTRL_X, 0x0F]; // Ctrl+X Ctrl+O

fn spawn_mbx_editor(home: &Path, extra_env: &[(&str, &str)], rc_prelude: &str) -> PtySession {
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
    PtySession::spawn(options).expect("editor shell spawn")
}

fn wait_prompt(session: &mut PtySession) {
    wait_all(session, &["> "]);
}

fn send_keyseq(session: &mut PtySession, keyseq: &[u8]) {
    session
        .write_all(keyseq, deadline(2))
        .expect("key sequence");
}

fn assert_no_insert_output(session: &mut PtySession) {
    let result = session.read_until(deadline(1), mbx_pty::DEFAULT_CAPTURE_LIMIT, |output| {
        visible_contains(output, "\nMBX_EDT:ok")
    });
    assert!(
        matches!(result, Err(mbx_pty::PtyError::Timeout(_))),
        "insert trigger executed before Enter: {:?}",
        result.map(|output| visible_text(&output))
    );
}

#[test]
fn insert_without_execute_puts_token_on_enter() {
    let home = TempHome::new("edt-e1");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_insert_output(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
}

#[test]
fn occupied_chord_is_not_overwritten() {
    let home = TempHome::new("edt-e2");
    let prelude = concat!(
        "_mbx_user_binding() { :; }\n",
        "bind -x '\"\\C-x\\C-y\": _mbx_user_binding'\n",
    );
    let mut session = spawn_mbx_editor(home.path(), &[], prelude);
    wait_prompt(&mut session);
    session
        .write_str("bind -X | grep -F '_mbx_user_binding'\n", deadline(2))
        .expect("query binding");
    wait_all(&mut session, &["_mbx_user_binding", "> "]);
    session
        .write_str(
            "[[ ${_MBX_EDITOR_INSERT_BOUND:-missing} == 0 ]] && printf 'MBX_EDT:refused\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_EDT:refused", "> "]);
}

#[test]
fn free_chord_installs_insert_binding() {
    let home = TempHome::new("edt-e2-free");
    let mut session = spawn_mbx_editor(
        home.path(),
        &[("MBX_EDITOR_INSERT_KEYSEQ", r"\C-x\C-o")],
        "",
    );
    wait_prompt(&mut session);
    session
        .write_str(
            "[[ ${_MBX_EDITOR_INSERT_BOUND:-missing} == 1 ]] && printf 'MBX_EDT:bound\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["\nMBX_EDT:bound", "> "]);
    send_keyseq(&mut session, ALT_KEYSEQ);
    assert_no_insert_output(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
}

#[test]
fn empty_line_trigger_inserts_without_submitting() {
    let home = TempHome::new("edt-e3");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_insert_output(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
}

#[test]
fn next_prompt_usable_after_insert_and_enter() {
    let home = TempHome::new("edt-e4-enter");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit insert");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
    session
        .write_str("printf 'MBX_EDT:next\\n'\n", deadline(2))
        .expect("follow-up");
    wait_all(&mut session, &["\nMBX_EDT:next", "> "]);
}

#[test]
fn next_prompt_usable_after_insert_and_ctrl_c() {
    let home = TempHome::new("edt-e4-cancel");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_all(&[CTRL_C], deadline(2)).expect("cancel");
    wait_all(&mut session, &["^C", "> "]);
    session
        .write_str("printf 'MBX_EDT:after_cancel\\n'\n", deadline(2))
        .expect("follow-up");
    wait_all(&mut session, &["\nMBX_EDT:after_cancel", "> "]);
}
