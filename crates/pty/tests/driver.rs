use mbx_pty::{
    CTRL_C, DEFAULT_CAPTURE_LIMIT, PtyError, PtySession, SpawnOptions, WinSize, contains_str,
    visible_contains, visible_text,
};
use std::env;
use std::ffi::OsString;
use std::time::{Duration, Instant};

fn deadline(seconds: u64) -> Instant {
    Instant::now() + Duration::from_secs(seconds)
}

fn path_env() -> OsString {
    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
}

fn spawn_bash() -> PtySession {
    let options = SpawnOptions::new("/bin/bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-i")
        .clear_env()
        .env("PATH", path_env())
        .env("HOME", "/tmp")
        .env("TERM", "xterm")
        .env("PS1", "PTY# ")
        .env("HISTFILE", "/dev/null")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .winsize(WinSize { rows: 24, cols: 80 });
    PtySession::spawn(options).expect("posix PTY spawn should succeed")
}

fn wait_for(session: &mut PtySession, needle: &str) -> Vec<u8> {
    match session.read_until(deadline(5), DEFAULT_CAPTURE_LIMIT, |output| {
        visible_contains(output, needle)
    }) {
        Ok(output) => output,
        Err(error) => panic!(
            "waiting for {needle:?} failed: {error} visible={:?}",
            match &error {
                PtyError::Timeout(captured) => visible_text(captured),
                _ => error.to_string(),
            }
        ),
    }
}

#[test]
fn interactive_bash_accepts_typed_input() {
    let mut session = spawn_bash();
    wait_for(&mut session, "PTY# ");
    session
        .write_str("printf 'PTY_OK\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nPTY_OK");
}

#[test]
fn read_until_times_out_without_hanging() {
    let mut session = spawn_bash();
    wait_for(&mut session, "PTY# ");
    let error = session
        .read_until(
            Instant::now() + Duration::from_millis(200),
            DEFAULT_CAPTURE_LIMIT,
            |output| contains_str(output, "NEVER_APPEARS"),
        )
        .expect_err("deadline should fire");
    assert!(matches!(error, PtyError::Timeout(_)));
}

#[test]
fn capture_bound_rejects_oversize_output() {
    let mut session = spawn_bash();
    let error = session
        .read_until(deadline(5), 8, |output| {
            contains_str(output, "NEVER_APPEARS")
        })
        .expect_err("startup output exceeds 8 bytes");
    assert!(matches!(error, PtyError::Oversize));
}

#[test]
fn resize_updates_reported_window_and_bash_columns() {
    let mut session = spawn_bash();
    wait_for(&mut session, "PTY# ");
    session
        .resize(WinSize { rows: 12, cols: 40 })
        .expect("resize");
    let size = session.winsize().expect("winsize");
    assert_eq!(size, WinSize { rows: 12, cols: 40 });
    session
        .write_str(
            "printf 'SIZE:%sx%s\\n' \"$LINES\" \"$COLUMNS\"\n",
            deadline(2),
        )
        .expect("write");
    wait_for(&mut session, "\nSIZE:12x40");
}

#[test]
fn ctrl_c_interrupts_foreground_job_and_returns_to_prompt() {
    let mut session = spawn_bash();
    wait_for(&mut session, "PTY# ");
    session
        .write_str(
            "sh -c 'printf \"PTY_RUNNING\\n\"; exec sleep 30'\n",
            deadline(2),
        )
        .expect("write");
    wait_for(&mut session, "\nPTY_RUNNING");
    session.write_all(&[CTRL_C], deadline(2)).expect("ctrl-c");
    wait_for(&mut session, "PTY# ");
    session
        .write_str("printf 'AFTER_INT\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nAFTER_INT");
}

#[test]
fn termios_probe_sees_isig_at_the_readline_prompt() {
    let mut session = spawn_bash();
    wait_for(&mut session, "PTY# ");
    let termios = session.termios().expect("tcgetattr on master");
    #[cfg(target_os = "linux")]
    const ISIG: u32 = 0x0001;
    #[cfg(target_os = "macos")]
    const ISIG: u64 = 0x0001;
    assert_eq!(termios.c_lflag & ISIG, ISIG);
    session
        .write_str("printf 'STTY:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("write");
    let output = wait_for(&mut session, "\nSTTY:");
    assert!(contains_str(&output, ":END"));
}
