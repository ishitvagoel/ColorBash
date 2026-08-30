mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, SpawnOptions, WinSize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const CTRL_B: u8 = 0x02;

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
fn highlight_install_sets_bound_flag_and_wraps_self_insert() {
    let home = TempHome::new("hl-pty0");
    let mut session = spawn_highlight_shell(home.path());
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
    let mut session = spawn_highlight_shell(home.path());
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
    let mut session = spawn_highlight_shell(home.path());
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
    let mut session = spawn_highlight_shell(home.path());
    wait_all(&mut session, &["> "]);
    session
        .write_str("printf 'HOSTILE:$`\\n'", deadline(2))
        .expect("type hostile line");
    wait_all(&mut session, &["printf 'HOSTILE:$`\\n'"]);
    session.write_str("\n", deadline(2)).expect("enter");
    wait_all(&mut session, &["\nHOSTILE:$`", "> "]);
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
