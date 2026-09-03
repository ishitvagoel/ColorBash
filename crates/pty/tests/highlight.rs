mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, Screen, SpawnOptions, WinSize, visible_text};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const CTRL_B: u8 = 0x02;

/// Reads until the shell goes quiet. Waiting on `"echo PREVIEW_OK"` can match
/// the preview row as soon as it is painted, before Readline redisplays the
/// prompt line, so Screen assertions drain instead of using a needle.
fn drain(session: &mut PtySession, seconds: u64) -> Vec<u8> {
    match session.read_until(deadline(seconds), mbx_pty::DEFAULT_CAPTURE_LIMIT, |_| false) {
        Ok(output) => output,
        Err(mbx_pty::PtyError::Timeout(captured)) => captured,
        Err(error) => panic!("draining the session failed: {error}"),
    }
}

fn spawn_highlight_shell(home: &Path, highlight: bool) -> PtySession {
    fs::write(
        home.join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
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
        .env("TERM", "xterm-256color")
        .env("USER", "mbx")
        .env("HISTFILE", "/dev/null")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("MBX_TEST_ROOT", workspace_root())
        .env("MBX_BIN", mbx_bin())
        .env("MBX_DISABLE_GIT", "1")
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 });
    if highlight {
        options = options.env("MBX_HIGHLIGHT", "1");
    }
    PtySession::spawn(options).expect("highlight shell spawn")
}

#[test]
fn highlight_install_sets_bound_flag_and_wraps_self_insert() {
    let home = TempHome::new("hl-pty0");
    let mut session = spawn_highlight_shell(home.path(), true);
    wait_all(&mut session, &["> "]);
    session
        .write_str(
            "[[ ${_MBX_HIGHLIGHT_BOUND:-missing} == 1 ]] && bind -X | grep -Fq '_mbx_highlight_self_insert' && printf 'MBX_HLT:bound\\n'\n",
            deadline(2),
        )
        .expect("status");
    wait_all(&mut session, &["MBX_HLT:bound\n", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn highlighted_line_executes_plain_bytes() {
    let home = TempHome::new("hl-pty1");
    let mut session = spawn_highlight_shell(home.path(), true);
    wait_all(&mut session, &["> "]);
    session
        .write_str("printf 'HL:plain\\n'", deadline(2))
        .expect("type");
    wait_all(&mut session, &["printf 'HL:plain\\n'"]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nHL:plain", "> "]);
    session
        .write_str("echo ok\n", deadline(2))
        .expect("next command");
    wait_all(&mut session, &["\nok", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn highlight_left_motion_keeps_plain_buffer_in_sync() {
    let home = TempHome::new("hl-pty2");
    let mut session = spawn_highlight_shell(home.path(), true);
    wait_all(&mut session, &["> "]);
    session.write_str("echo ab", deadline(2)).expect("type");
    wait_all(&mut session, &["echo ab"]);
    session.write_all(&[CTRL_B], deadline(2)).expect("backward");
    session.write_str("X", deadline(2)).expect("insert");
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\naXb", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn hostile_highlighted_line_executes_plain_bytes() {
    let home = TempHome::new("hl-pty3");
    let mut session = spawn_highlight_shell(home.path(), true);
    wait_all(&mut session, &["> "]);
    session
        .write_str("printf 'HOSTILE:$`\\n'", deadline(2))
        .expect("type hostile line");
    wait_all(&mut session, &["printf 'HOSTILE:$`\\n'"]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nHOSTILE:$`", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

// H-4 (docs/hlt-comp-review-close-plan.md): with `MBX_HIGHLIGHT` unset the
// installer must not run at all — no highlight widgets in `bind -X`, the
// bound flag stays 0, and typing/Enter stay stock. The needles are built with
// printf indirection so the echoed command line cannot forge a match
// (M-019/M-073).
#[test]
fn highlight_unset_installs_no_widgets() {
    let home = TempHome::new("hl-pty6");
    let mut session = spawn_highlight_shell(home.path(), false);
    wait_all(&mut session, &["> "]);
    session
        .write_str(
            "bind -X | grep -Fq _mbx_highlight_ && printf 'MBX_HLT:%s\\n' widgets || printf 'MBX_HLT:%s\\n' absent\n",
            deadline(2),
        )
        .expect("widget check");
    session
        .write_str(
            "[[ ${_MBX_HIGHLIGHT_BOUND:-0} == 1 ]] && printf 'MBX_FLG:%s\\n' set || printf 'MBX_FLG:%s\\n' unset\n",
            deadline(2),
        )
        .expect("flag check");
    wait_all(&mut session, &["MBX_HLT:absent", "MBX_FLG:unset", "> "]);
    session.write_str("echo stock", deadline(2)).expect("type");
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nstock", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

// M-064's prevention rule (MISTAKES.md): the rendered line must contain no
// caret-encoded control characters — not just round-trip to the plain bytes.
// Under ADR 0015 the edit buffer stays permanently plain and styled bytes
// paint on a reserved row below the prompt, so this pins that invariant: if
// styling ever reaches `READLINE_LINE` again (the pre-0015 design), Readline
// redisplay emits literal `^A`/`^[`/`^B` sequences and this test fails. The
// typed line must contain tokens the lexer actually styles (keywords, quotes,
// operators); plain words render identically either way and would prove
// nothing.
#[test]
fn typed_line_renders_without_caret_control_leftovers() {
    let home = TempHome::new("hl-pty7");
    let mut session = spawn_highlight_shell(home.path(), true);
    wait_all(&mut session, &["> "]);
    session.write_str("echo \"hi\"", deadline(2)).expect("type");
    // Wait on a fragment that appears in both the plain and the M-064-garbled
    // render, so a flip of the color decision reaches the caret assertion
    // below instead of dying as a wait timeout.
    let captured = wait_all(&mut session, &["\"hi\""]);
    let text = visible_text(&captured);
    assert!(
        !text.contains("^A") && !text.contains("^B") && !text.contains("^["),
        "the rendered line leaked caret-encoded control characters (M-064 \
         regression: styling reached READLINE_LINE without a technique \
         Readline hides); visible text: {text:?}"
    );
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nhi", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

/// A counting wrapper standing in for `MBX_BIN`: it counts every invocation
/// that is not the one-time `serve --stdio` coprocess spawn, then execs the
/// real helper so the session still works. Used to prove, structurally
/// rather than by timing, that a wire-path keystroke never forks the helper
/// binary (T1-4; the roadmap's "no external command on a cache-hit
/// keystroke" budget).
fn write_counting_bin_shim(home: &Path, real_bin: &Path, count_file: &Path) -> std::path::PathBuf {
    let shim = home.join("mbx-counting");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n  serve) ;;\n  *) printf 'x' >> {count} ;;\nesac\nexec {real} \"$@\"\n",
            count = shell_quote(count_file),
            real = shell_quote(real_bin),
        ),
    )
    .expect("counting shim");
    let mut perms = fs::metadata(&shim).expect("shim metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&shim, perms).expect("shim chmod");
    shim
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

#[test]
fn wire_highlight_forks_no_helper_process_per_keystroke() {
    let home = TempHome::new("hl-pty4");
    let count_file = home.path().join("spawn-count");
    fs::write(&count_file, "").expect("seed count file");
    let shim = write_counting_bin_shim(home.path(), &mbx_bin(), &count_file);

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
            .env("TERM", "xterm-256color")
            .env("USER", "mbx")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", &shim)
            .env("MBX_HIGHLIGHT", "1")
            .env("MBX_DISABLE_GIT", "1")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("counting-shim shell spawn");
    wait_all(&mut session, &["> "]);
    session
        .write_str(
            "[[ ${_MBX_ENGINE_READY:-0} == 1 ]] && printf 'MBX_HLT:engine-ready\\n'\n",
            deadline(2),
        )
        .expect("check engine readiness");
    wait_all(&mut session, &["MBX_HLT:engine-ready\n", "> "]);

    session
        .write_str("echo the_quick_brown_fox", deadline(2))
        .expect("type many keystrokes");
    wait_all(&mut session, &["echo the_quick_brown_fox"]);

    let spawn_count = fs::read_to_string(&count_file).expect("read count file");
    assert!(
        spawn_count.is_empty(),
        "expected zero non-serve mbx invocations while the coprocess is ready \
         (each keystroke should use the HIGHLIGHT/STYLED wire frame, not a \
         process spawn); got {} invocation(s)",
        spawn_count.len()
    );

    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nthe_quick_brown_fox", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn cli_fallback_highlight_does_fork_the_helper_per_keystroke() {
    // Sanity check for the shim itself and the contrast case: with the
    // coprocess explicitly disabled, the CLI spawn fallback is the only
    // path, so the counter must increase. If this test ever fails, the shim
    // (not the product) is broken and the sibling wire-path test proves
    // nothing.
    let home = TempHome::new("hl-pty5");
    let count_file = home.path().join("spawn-count");
    fs::write(&count_file, "").expect("seed count file");
    let shim = write_counting_bin_shim(home.path(), &mbx_bin(), &count_file);

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
            .env("TERM", "xterm-256color")
            .env("USER", "mbx")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", &shim)
            .env("MBX_HIGHLIGHT", "1")
            .env("MBX_DISABLE_GIT", "1")
            .env("MBX_DISABLE_RENDERER", "1")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("counting-shim shell spawn");
    wait_all(&mut session, &["> "]);

    session.write_str("echo ab", deadline(2)).expect("type");
    wait_all(&mut session, &["echo ab"]);

    let spawn_count = fs::read_to_string(&count_file).expect("read count file");
    assert!(
        !spawn_count.is_empty(),
        "the CLI fallback must still fork the helper once per keystroke \
         when no coprocess is available"
    );

    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nab", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn highlight_preview_row_paints_sgr_below_an_intact_prompt() {
    let home = TempHome::new("hl-pty-preview");
    let mut session = spawn_highlight_shell(home.path(), true);
    let mut transcript: Vec<u8> = Vec::new();
    macro_rules! step {
        ($needles:expr) => {{
            let chunk = wait_all(&mut session, $needles);
            transcript.extend_from_slice(&chunk);
        }};
    }

    step!(&["> "]);
    session.write_str("true", deadline(2)).expect("type");
    transcript.extend_from_slice(&drain(&mut session, 2));

    let mut screen = Screen::new(24, 80);
    screen.apply(&transcript);
    let lines = screen.lines();
    assert!(
        lines.iter().any(|line| line.contains("> true")),
        "the prompt line must keep the plain edit buffer; screen was:\n{}",
        lines.join("\n")
    );
    assert!(
        lines.iter().any(|line| line.trim() == "true"),
        "a reserved row below the prompt must show the styled copy; screen was:\n{}",
        lines.join("\n")
    );
    let text = String::from_utf8_lossy(&transcript);
    assert!(
        text.contains("\u{1b}[1;34m"),
        "a color-capable tty must receive SGR for the true keyword"
    );
    assert!(
        !text.contains("^A") && !text.contains("^B"),
        "Readline must not caret-render SOH/STX; READLINE_LINE stays plain"
    );

    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}
