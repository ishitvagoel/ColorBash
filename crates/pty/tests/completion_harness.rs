mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, SpawnOptions, WinSize};
use std::fs;
use std::path::Path;

const TAB: u8 = 0x09;

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
    let mut session = spawn_mbx_shell(home.path(), &[], "");
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
            "\nMBX_COMP:mbx_comp_probe mbx_co",
            ":mbx_comp_candidate",
            "> ",
        ],
    );
}

#[test]
fn stock_ls_completion_is_not_wrapped() {
    let home = TempHome::new("comp-h3");
    let mut session = spawn_mbx_shell(home.path(), &[], "");
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
    let mut session = spawn_mbx_shell(home.path(), &[], "");
    wait_prompt(&mut session);
    session
        .write_str("ls MBX_COMP_U", deadline(2))
        .expect("type prefix");
    send_tab(&mut session);
    session.write_str("\n", deadline(2)).expect("submit");
    wait_all(&mut session, &["MBX_COMP_UNIQUE", "> "]);
}
