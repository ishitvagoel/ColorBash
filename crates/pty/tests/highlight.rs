mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, SpawnOptions, WinSize};
use std::fs;
use std::path::Path;

fn spawn_highlight_shell(home: &Path) -> PtySession {
    fs::write(
        home.join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("rcfile");
    PtySession::spawn(
        SpawnOptions::new("/bin/bash")
            .arg("--noprofile")
            .arg("--rcfile")
            .arg(home.join("rc.bash"))
            .arg("-i")
            .clear_env()
            .env("PATH", path_env())
            .env("HOME", home)
            .env("TERM", "xterm-256color")
            .env("USER", "mbx")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", mbx_bin())
            .env("MBX_HIGHLIGHT", "1")
            .env("MBX_DISABLE_GIT", "1")
            .cwd(home)
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("highlight shell spawn")
}

#[test]
fn highlighted_line_executes_plain_bytes() {
    let home = TempHome::new("hl-pty1");
    let mut session = spawn_highlight_shell(home.path());
    wait_all(&mut session, &["> "]);
    session
        .write_str("printf 'HL:plain\\n'", deadline(2))
        .expect("type");
    wait_all(&mut session, &["printf 'HL:plain\\n'"]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nHL:plain", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}
