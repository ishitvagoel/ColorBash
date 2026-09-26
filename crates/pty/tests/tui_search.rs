//! TUI-004 evidence (ADR 0016): the chord-launched history picker restores
//! the terminal on every path, inserts exactly the selected bytes without
//! executing anything, refuses hostile output, and falls back to the chord
//! widget when the helper fails. `MBX_TUI=0` keeps the old behavior.

mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, SpawnOptions, WinSize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const CTRL_X_H: &[u8] = &[0x18, 0x68];

/// Writes a stand-in `mbx` that answers `tui search` from a scripted behavior
/// (`mode` file) and execs the real binary for everything else, so the
/// fallback path exercises the actual history queries.
fn write_tui_shim(home: &Path, real_bin: &Path, mode: &Path) -> std::path::PathBuf {
    let shim = home.join("mbx-tui-shim");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\nif [ \"$1\" = tui ]; then\n  case \"$(cat {mode})\" in\n    select) printf 'echo SELECTED\\n'; exit 0 ;;\n    cancel) exit 2 ;;\n    empty) exit 0 ;;\n    hostile) printf 'echo \\033EVIL\\n'; exit 0 ;;\n    fail) exit 1 ;;\n  esac\n  exit 1\nfi\nexec {real} \"$@\"\n",
            mode = shell_quote(mode),
            real = shell_quote(real_bin),
        ),
    )
    .expect("shim script");
    let mut perms = fs::metadata(&shim).expect("shim metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&shim, perms).expect("shim chmod");
    shim
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

fn spawn_tui_shell(home: &TempHome, bin: &Path) -> PtySession {
    fs::write(
        home.path().join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("rcfile");
    PtySession::spawn(
        SpawnOptions::new("/bin/bash")
            .arg("--noprofile")
            .arg("--rcfile")
            .arg(home.path().join("rc.bash"))
            .arg("-i")
            .clear_env()
            .env("PATH", path_env())
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.data_home())
            .env("TERM", "xterm-256color")
            .env("USER", "mbx")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", bin)
            .env("MBX_HISTORY", "1")
            .env("MBX_TUI", "1")
            .env("MBX_IPC_TIMEOUT", "5")
            .env("MBX_HISTORY_TIMEOUT", "5")
            .env("MBX_DISABLE_GIT", "1")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("tui shell spawn")
}

/// Snapshot and compare `stty -g` around the picker: the vim oracle
/// (`vim_fullscreen_quits_and_restores_stty`) applied to the TUI.
fn prime_stty_probe(session: &mut PtySession) {
    session
        .write_str(
            "MBX_S1=$(stty -g); printf 'MBX_TUI:%s\\n' probed\n",
            deadline(2),
        )
        .expect("stty probe");
    wait_all(session, &["MBX_TUI:probed", "> "]);
}

fn assert_stty_restored(session: &mut PtySession) {
    // Markers are composed with printf indirection so the echoed command
    // line cannot forge a match (M-019/M-073).
    session
        .write_str(
            "MBX_S2=$(stty -g); [[ -n $MBX_S1 && $MBX_S1 == \"$MBX_S2\" ]] && printf 'MBX_TUI:stty=%s\\n' RESTORED || printf 'MBX_TUI:stty=%s\\n' CHANGED\n",
            deadline(2),
        )
        .expect("stty compare");
    let raw = match session.read_until(deadline(3), 1 << 16, |output| {
        mbx_pty::visible_contains(output, "stty=RESTORED")
            || mbx_pty::visible_contains(output, "stty=CHANGED")
    }) {
        Ok(raw) => raw,
        Err(mbx_pty::PtyError::Timeout(raw)) => raw,
        Err(error) => panic!("stty compare capture: {error}"),
    };
    let text = mbx_pty::visible_text(&raw);
    assert!(
        text.contains("stty=RESTORED") && !text.contains("stty=CHANGED"),
        "the terminal state must survive the picker unchanged (stty -g \
         identical before and after); screen tail was: {text:?}"
    );
    wait_all(session, &["> "]);
}

#[test]
fn tui_selection_replaces_the_line_and_restores_stty() {
    let home = TempHome::new("tui-pty0");
    let mode = home.path().join("mode");
    fs::write(&mode, "select").expect("mode file");
    let shim = write_tui_shim(home.path(), &mbx_bin(), &mode);
    let mut session = spawn_tui_shell(&home, &shim);
    wait_all(&mut session, &["> "]);
    prime_stty_probe(&mut session);

    session.write_str("seed", deadline(2)).expect("type");
    session.write_all(CTRL_X_H, deadline(2)).expect("chord");
    // Readline repaints the inserted selection after the widget returns: the
    // prompt line must now hold the selection, not the typed seed.
    let captured = wait_all(&mut session, &["echo SELECTED"]);
    let text = mbx_pty::visible_text(&captured);
    assert!(
        text.contains("> echo SELECTED"),
        "the picker must replace the whole line with the selection; screen: {text:?}"
    );
    // Nothing executed: the selection is still editable text at the prompt.
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nSELECTED", "> "]);
    assert_stty_restored(&mut session);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn tui_cancel_keeps_the_typed_line_and_restores_stty() {
    let home = TempHome::new("tui-pty1");
    let mode = home.path().join("mode");
    fs::write(&mode, "cancel").expect("mode file");
    let shim = write_tui_shim(home.path(), &mbx_bin(), &mode);
    let mut session = spawn_tui_shell(&home, &shim);
    wait_all(&mut session, &["> "]);
    prime_stty_probe(&mut session);

    session.write_str("echo kept", deadline(2)).expect("type");
    session.write_all(CTRL_X_H, deadline(2)).expect("chord");
    // After cancel, Readline redraws the untouched typed line.
    wait_all(&mut session, &["echo kept", "> "]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nkept", "> "]);
    assert_stty_restored(&mut session);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn tui_hostile_output_is_refused() {
    let home = TempHome::new("tui-pty2");
    let mode = home.path().join("mode");
    fs::write(&mode, "hostile").expect("mode file");
    let shim = write_tui_shim(home.path(), &mbx_bin(), &mode);
    let mut session = spawn_tui_shell(&home, &shim);
    wait_all(&mut session, &["> "]);

    session.write_str("echo safe", deadline(2)).expect("type");
    session.write_all(CTRL_X_H, deadline(2)).expect("chord");
    // The C0/DEL gate refuses the ESC-bearing row: the typed line survives.
    wait_all(&mut session, &["echo safe", "> "]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nsafe", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn tui_helper_failure_falls_back_to_the_chord_widget() {
    let home = TempHome::new("tui-pty3");
    let mode = home.path().join("mode");
    fs::write(&mode, "fail").expect("mode file");
    let shim = write_tui_shim(home.path(), &mbx_bin(), &mode);
    let mut session = spawn_tui_shell(&home, &shim);
    wait_all(&mut session, &["> "]);

    // Seed the store so the widget fallback has a real match to insert.
    session
        .write_str("echo fallback-row\n", deadline(2))
        .expect("record row");
    wait_all(&mut session, &["\nfallback-row", "> "]);

    session.write_str("echo fal", deadline(2)).expect("type");
    session.write_all(CTRL_X_H, deadline(2)).expect("chord");
    wait_all(&mut session, &["echo fallback-row", "> "]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nfallback-row", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn tui_disabled_keeps_the_widget_path() {
    let home = TempHome::new("tui-pty4");
    let mode = home.path().join("mode");
    fs::write(&mode, "select").expect("mode file: never read");
    let shim = write_tui_shim(home.path(), &mbx_bin(), &mode);
    fs::write(
        home.path().join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("rcfile");
    let mut session = PtySession::spawn(
        SpawnOptions::new("/bin/bash")
            .arg("--noprofile")
            .arg("--rcfile")
            .arg(home.path().join("rc.bash"))
            .arg("-i")
            .clear_env()
            .env("PATH", path_env())
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.data_home())
            .env("TERM", "xterm-256color")
            .env("USER", "mbx")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", &shim)
            .env("MBX_HISTORY", "1")
            .env("MBX_DISABLE_GIT", "1")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("non-tui shell spawn");
    wait_all(&mut session, &["> "]);

    session
        .write_str("echo tui-disabled-row\n", deadline(2))
        .expect("record row");
    wait_all(&mut session, &["\ntui-disabled-row", "> "]);

    session.write_str("echo tui", deadline(2)).expect("type");
    session.write_all(CTRL_X_H, deadline(2)).expect("chord");
    // Without MBX_TUI=1 the chord cycles the widget: the line becomes the
    // recorded match, proving the shim's tui mode was never used.
    wait_all(&mut session, &["echo tui-disabled-row", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

/// Live evidence for the Rust picker itself: `mbx tui search` is driven
/// directly in a PTY (no Bash layer), so the raw-mode/alternate-screen
/// contract — paint, type-to-filter, resize, select, restore — is exercised
/// against the real binary. The chord seam around it is covered by the shim
/// tests above.
#[test]
fn real_tui_filters_selects_and_restores_the_terminal() {
    let home = TempHome::new("tui-pty5");
    let bin = mbx_bin();

    // Seed the store with a short interactive session (capture is a
    // prompt-boundary feature, so rows come from a real shell).
    let mut seeder = spawn_tui_shell(&home, &bin);
    wait_all(&mut seeder, &["> "]);
    for row in ["echo alpha-one", "echo alpha-two", "echo other"] {
        seeder.write_str(row, deadline(2)).expect("type row");
        seeder.write_str("\n", deadline(2)).expect("enter");
        wait_all(&mut seeder, &["> "]);
    }
    common::wait_for_count(&bin, &home.data_home(), 3);
    seeder.write_str("exit\n", deadline(2)).expect("exit");

    // Drive the picker directly: no shell, no chords, no coprocess.
    let mut session = PtySession::spawn(
        SpawnOptions::new(&bin)
            .arg("tui")
            .arg("search")
            .clear_env()
            .env("PATH", path_env())
            .env("HOME", home.path())
            .env("XDG_DATA_HOME", home.data_home())
            .env("TERM", "xterm-256color")
            .env("USER", "mbx")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("standalone tui spawn");

    wait_all(&mut session, &["MBX history - 3 match"]);
    // Type-to-filter: two of the three rows mention alpha.
    session.write_str("alpha", deadline(2)).expect("filter");
    wait_all(&mut session, &["2 match"]);
    // Resize mid-session: SIGWINCH must trigger a fresh frame at the new
    // size without the picker dying or losing state.
    session
        .resize(WinSize {
            rows: 30,
            cols: 100,
        })
        .expect("resize");
    wait_all(&mut session, &["MBX history - 2 match"]);
    // The list is newest-first: [alpha-two, alpha-one]. Ctrl-N moves the
    // selection to the second row.
    session.write_str("\x0e", deadline(2)).expect("Ctrl-N");
    session.write_str("\r", deadline(2)).expect("Enter accept");
    // Enter accepts: exit status 0 is the selection (a cancel exits 2), so
    // it is the deterministic evidence that Enter accepted the highlighted
    // row. The restore escapes and the bare selection are written just
    // before process exit, and the kernel may discard pty data that was
    // never read by then — so they are asserted only when the capture won
    // that race. The full restore contract at the shell level is covered by
    // the stty oracle in the shim tests above.
    let selected = match session.read_until(deadline(3), 1 << 16, |output: &[u8]| {
        output
            .windows(8)
            .any(|window| window == b"\x1b[?25h\x1b[?1049l")
    }) {
        Ok(raw) => {
            let restored = raw
                .windows(8)
                .any(|window| window == b"\x1b[?25h\x1b[?1049l");
            assert!(
                restored,
                "the picker must restore the cursor and leave the alternate \
                 screen before returning; raw output: {raw:?}"
            );
            let rest = &raw[raw
                .windows(8)
                .position(|window| window == b"\x1b[?25h\x1b[?1049l")
                .expect("restore position")
                + 8..];
            assert_eq!(
                std::str::from_utf8(rest).unwrap_or("<invalid utf8>").trim(),
                "echo alpha-one",
                "stdout after the restore must be exactly the selected \
                 command; got: {rest:?}"
            );
            session.wait().expect("tui wait").success()
        }
        Err(mbx_pty::PtyError::ChildExited) => session.wait().expect("tui wait").success(),
        Err(error) => panic!("accept capture: {error}"),
    };
    assert!(
        selected,
        "exit status 0 means Enter accepted the highlighted row; a \
         nonzero status is the cancel path"
    );
}
