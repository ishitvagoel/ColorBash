//! ONBD-001 / ONBD-003 evidence: the first source is not silent, the welcome
//! shows once per config directory, and the inert history-search chord
//! explains itself once without touching the line buffer.

mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, SpawnOptions, WinSize};

fn spawn_shell(home: &TempHome, history: bool) -> PtySession {
    std::fs::write(
        home.path().join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("rcfile");
    let mut options = SpawnOptions::new("/bin/bash")
        .arg("--noprofile")
        .arg("--rcfile")
        .arg(home.path().join("rc.bash"))
        .arg("-i")
        .clear_env()
        .env("PATH", path_env())
        .env("HOME", home.path())
        .env("TERM", "xterm-256color")
        .env("USER", "mbx")
        .env("HISTFILE", "/dev/null")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("MBX_TEST_ROOT", workspace_root())
        .env("MBX_BIN", mbx_bin())
        .env("MBX_DISABLE_GIT", "1")
        .cwd(home.path())
        .winsize(WinSize { rows: 24, cols: 80 });
    if history {
        options = options.env("MBX_HISTORY", "1");
    }
    PtySession::spawn(options).expect("onboarding shell spawn")
}

#[test]
fn first_source_prints_banner_once_and_marks_the_config_dir() {
    let home = TempHome::new("onbd-pty0");
    let mut session = spawn_shell(&home, false);
    // The banner precedes the first prompt; both needles land in one read.
    wait_all(&mut session, &["MBX loaded.", "Something look off?", "> "]);

    // The marker is created beside the user config so the banner is once per
    // config directory, not once per shell.
    session
        .write_str(
            "if [[ -f $HOME/.config/mbx/first-run-shown ]]; then printf 'MBX_ONBD:%s\\n' marker; fi\n",
            deadline(2),
        )
        .expect("marker check");
    wait_all(&mut session, &["MBX_ONBD:marker", "> "]);

    // A second source in the same session must not repeat the banner.
    session
        .write_str(
            "source \"$MBX_TEST_ROOT/bash/init.bash\"; printf 'MBX_ONBD:%s\\n' done\n",
            deadline(2),
        )
        .expect("second source");
    let tail = wait_all(&mut session, &["MBX_ONBD:done", "> "]);
    let text = mbx_pty::visible_text(&tail);
    assert!(
        !text.contains("MBX loaded."),
        "the welcome banner must show once per config directory, not on every \
         source; tail was: {text:?}"
    );
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn inert_search_chord_hints_once_and_keeps_the_line() {
    let home = TempHome::new("onbd-pty1");
    let mut session = spawn_shell(&home, false);
    wait_all(&mut session, &["> "]);

    // Type a command, then press the search chord with history off: the hint
    // must appear, the typed line must survive, and Enter must run it.
    session.write_str("echo kept", deadline(2)).expect("type");
    wait_all(&mut session, &["echo kept"]);
    session
        .write_all(&[0x18, 0x68], deadline(2))
        .expect("Ctrl-X h");
    // Wait for the hint *and* the post-widget redraw together: the hint
    // reaches the terminal during the widget, and Readline only repaints the
    // prompt line after it returns.
    let captured = wait_all(&mut session, &["history search is off", "echo kept"]);
    let text = mbx_pty::visible_text(&captured);
    assert!(
        text.contains("history search is off") && text.contains("echo kept"),
        "the inert chord must leave the typed line intact; screen was: {text:?}"
    );
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nkept", "> "]);

    // The hint is once per session: the second chord stays silent.
    session
        .write_all(&[0x18, 0x68], deadline(2))
        .expect("Ctrl-X h");
    session
        .write_str("printf 'MBX_ONBD:%s\\n' second\n", deadline(2))
        .expect("second marker");
    let tail = wait_all(&mut session, &["MBX_ONBD:second", "> "]);
    let text = mbx_pty::visible_text(&tail);
    assert!(
        !text.contains("history search is off"),
        "the inert-chord hint must print only once per session; tail was: {text:?}"
    );
    session.write_str("exit\n", deadline(2)).expect("exit");
}
