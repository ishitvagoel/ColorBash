mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{CTRL_C, CTRL_Z, PtySession, SpawnOptions, WinSize, visible_contains, visible_text};
use std::fs;
use std::path::Path;

const CTRL_X: u8 = 0x18;
const CTRL_Y: u8 = 0x19;
const CTRL_B: u8 = 0x02;
const DEFAULT_KEYSEQ: &[u8] = &[CTRL_X, CTRL_Y];
const ALT_KEYSEQ: &[u8] = &[CTRL_X, 0x0F]; // Ctrl+X Ctrl+O
const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";
const TOKEN_ENV: &[(&str, &str)] = &[("MBX_EDITOR_INSERT_TOKEN", "MBX_EDT_TOKEN")];

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

fn start_sleep(session: &mut PtySession) {
    session
        .write_str(
            "sh -c 'printf \"MBX_PTY:running\\n\"; exec sleep 30'\n",
            deadline(2),
        )
        .expect("write");
    wait_all(session, &["\nMBX_PTY:running"]);
}

fn send_bracketed_paste(session: &mut PtySession, text: &str) {
    session
        .write_all(BRACKETED_PASTE_START, deadline(2))
        .expect("paste start");
    session.write_str(text, deadline(2)).expect("paste body");
    session
        .write_all(BRACKETED_PASTE_END, deadline(2))
        .expect("paste end");
}

fn send_ctrl_b(session: &mut PtySession, count: usize) {
    for _ in 0..count {
        session.write_all(&[CTRL_B], deadline(2)).expect("ctrl-b");
    }
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
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_EDT:after_cancel\\n'\n", deadline(2))
        .expect("follow-up");
    wait_all(&mut session, &["\nMBX_EDT:after_cancel", "> "]);
}

#[test]
fn vi_insert_mode_inserts_without_execute() {
    let home = TempHome::new("edt-m1");
    let mut session = spawn_mbx_editor(home.path(), &[], "set -o vi\n");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_insert_output(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
}

#[test]
fn bracketed_paste_does_not_execute_until_enter() {
    let home = TempHome::new("edt-m2");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    send_bracketed_paste(&mut session, "printf 'MBX_EDT:ok\\n'");
    assert_no_insert_output(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
}

#[test]
fn resize_after_insert_still_runs_token() {
    let home = TempHome::new("edt-m3");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session
        .resize(WinSize { rows: 16, cols: 64 })
        .expect("resize");
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
    session
        .write_str("printf 'MBX_EDT:resized\\n'\n", deadline(2))
        .expect("follow-up");
    wait_all(&mut session, &["\nMBX_EDT:resized", "> "]);
}

#[test]
fn ctrl_z_then_insert_still_works() {
    let home = TempHome::new("edt-m4");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    start_sleep(&mut session);
    session.write_all(&[CTRL_Z], deadline(2)).expect("ctrl-z");
    wait_prompt(&mut session);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit insert");
    wait_all(&mut session, &["\nMBX_EDT:ok", "> "]);
    session
        .write_str("kill %1 2>/dev/null || true\n", deadline(2))
        .expect("cleanup");
}

#[test]
fn mid_line_insert_preserves_prefix_and_suffix() {
    let home = TempHome::new("edt-b1");
    let mut session = spawn_mbx_editor(home.path(), TOKEN_ENV, "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s\\n' 'XX'", deadline(2))
        .expect("type prefix and suffix");
    send_ctrl_b(&mut session, 3);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["\nGOT:MBX_EDT_TOKENXX", "> "]);
}

#[test]
fn cursor_lands_after_inserted_token() {
    let home = TempHome::new("edt-b2");
    let mut session = spawn_mbx_editor(home.path(), TOKEN_ENV, "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s\\n' '", deadline(2))
        .expect("type prefix");
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session
        .write_str("YY'\n", deadline(2))
        .expect("type suffix and submit");
    wait_all(&mut session, &["\nGOT:MBX_EDT_TOKENYY", "> "]);
}

#[test]
fn quoted_insert_is_data_not_execution() {
    let home = TempHome::new("edt-b3");
    let mut session = spawn_mbx_editor(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s\\n' '", deadline(2))
        .expect("type prefix");
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    assert_no_insert_output(&mut session);
    session
        .write_str("'\n", deadline(2))
        .expect("close quote and submit");
    let output = wait_all(&mut session, &["\nGOT:printf", "> "]);
    let text = visible_text(&output);
    assert!(
        !text.contains("\nMBX_EDT:ok\n"),
        "inserted command executed as a standalone line: {text}"
    );
    assert!(
        text.contains("MBX_EDT:ok"),
        "token text missing from GOT output: {text}"
    );
}

#[test]
fn multiline_ps2_insert_preserves_exact_bytes() {
    let home = TempHome::new("edt-b4");
    let mut session = spawn_mbx_editor(
        home.path(),
        &[
            ("MBX_EDITOR_INSERT_TOKEN", "MBX_EDT_TOKEN"),
            ("PS2", "CONT> "),
        ],
        "",
    );
    wait_prompt(&mut session);
    session
        .write_str("printf 'GOT:%s\\n' '\n", deadline(2))
        .expect("open quoted line");
    wait_all(&mut session, &["CONT> "]);
    send_keyseq(&mut session, DEFAULT_KEYSEQ);
    session
        .write_str("'\n", deadline(2))
        .expect("close and submit");
    wait_all(&mut session, &["\nGOT:", "MBX_EDT_TOKEN", "> "]);
}
