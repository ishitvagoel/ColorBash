use mbx_pty::{
    CTRL_C, CTRL_Z, DEFAULT_CAPTURE_LIMIT, PtyError, PtySession, SpawnOptions, WinSize,
    visible_contains, visible_text,
};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn deadline(seconds: u64) -> Instant {
    Instant::now() + Duration::from_secs(seconds)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn mbx_bin() -> PathBuf {
    if let Some(path) = env::var_os("MBX_TEST_BIN") {
        return PathBuf::from(path);
    }
    let root = workspace_root();
    let debug = root.join("target/debug/mbx");
    let release = root.join("target/release/mbx");
    if debug.is_file() {
        debug
    } else if release.is_file() {
        release
    } else {
        panic!("mbx binary missing; build the workspace before PTY foundation tests");
    }
}

fn path_env() -> OsString {
    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
}

fn write_rc(dir: &Path) {
    fs::write(
        dir.join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("rcfile");
}

fn spawn_mbx(bin: &Path, home: &Path) -> PtySession {
    write_rc(home);
    let options = SpawnOptions::new("/bin/bash")
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
        .env("MBX_BIN", bin)
        .env("MBX_COLOR", "never")
        .env("MBX_ICONS", "never")
        .env("MBX_DISABLE_GIT", "1")
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 });
    PtySession::spawn(options).expect("MBX PTY spawn")
}

fn wait_for(session: &mut PtySession, needle: &str) -> Vec<u8> {
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |output| {
        visible_contains(output, needle)
    }) {
        Ok(output) => output,
        Err(error) => panic!(
            "waiting for {needle:?} failed: {error} output={:?}",
            captured_preview(&error)
        ),
    }
}

fn captured_preview(error: &PtyError) -> String {
    match error {
        PtyError::Timeout(captured) => visible_text(captured),
        _ => error.to_string(),
    }
}

fn wait_prompt(session: &mut PtySession) -> Vec<u8> {
    wait_for(session, "> ")
}

fn start_sleep(session: &mut PtySession) {
    session
        .write_str(
            "sh -c 'printf \"MBX_PTY:running\\n\"; exec sleep 30'\n",
            deadline(2),
        )
        .expect("write");
    wait_for(session, "\nMBX_PTY:running");
}

fn wait_all(session: &mut PtySession, needles: &[&str]) -> Vec<u8> {
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |output| {
        needles
            .iter()
            .all(|needle| visible_contains(output, needle))
    }) {
        Ok(output) => output,
        Err(error) => panic!(
            "waiting for {needles:?} failed: {error} output={:?}",
            captured_preview(&error)
        ),
    }
}

struct TempHome(PathBuf);

impl TempHome {
    fn new() -> Self {
        let dir = env::temp_dir().join(format!("mbx-pty-{}-{}", std::process::id(), {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        }));
        fs::create_dir_all(&dir).expect("temp home");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn prompt_lifecycle_renders_and_runs_commands() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_PTY:ok\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_PTY:ok");
    wait_prompt(&mut session);
}

#[test]
fn missing_helper_falls_back_to_usable_prompt() {
    let home = TempHome::new();
    let mut session = spawn_mbx(Path::new("/definitely/missing/mbx"), home.path());
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_PTY:fallback\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_PTY:fallback");
}

#[test]
fn helper_crash_degrades_without_disabling_the_shell() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    session
        .write_str(
            "if [[ -n ${_MBX_ENGINE_CHILD_PID:-} ]]; then kill \"$_MBX_ENGINE_CHILD_PID\"; fi; true; printf 'MBX_PTY:recovered\\n'\n",
            deadline(2),
        )
        .expect("write");
    wait_for(&mut session, "\nMBX_PTY:recovered");
    wait_prompt(&mut session);
}

#[test]
fn ctrl_c_restores_a_usable_prompt() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    start_sleep(&mut session);
    session.write_all(&[CTRL_C], deadline(2)).expect("ctrl-c");
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_PTY:after_int\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_PTY:after_int");
}

#[test]
fn ctrl_z_stops_a_job_and_returns_to_the_prompt() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    start_sleep(&mut session);
    session.write_all(&[CTRL_Z], deadline(2)).expect("ctrl-z");
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_PTY:after_stop\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_PTY:after_stop");
    session
        .write_str("kill %1 2>/dev/null || true\n", deadline(2))
        .expect("cleanup");
}

#[test]
fn resize_updates_lines_and_columns() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    session
        .resize(WinSize { rows: 16, cols: 64 })
        .expect("resize");
    session
        .write_str(
            "printf 'MBX_PTY:size:%sx%s\\n' \"$LINES\" \"$COLUMNS\"\n",
            deadline(2),
        )
        .expect("write");
    wait_for(&mut session, "\nMBX_PTY:size:16x64");
}

#[test]
fn ctrl_c_and_resize_preserve_stty_settings() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    session
        .write_str("printf 'STTY1:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("write");
    let first = wait_all(&mut session, &["\nSTTY1:", ":END"]);
    let before = extract_marked(&first, "STTY1:", ":END");
    start_sleep(&mut session);
    session.write_all(&[CTRL_C], deadline(2)).expect("ctrl-c");
    wait_prompt(&mut session);
    session
        .resize(WinSize { rows: 20, cols: 72 })
        .expect("resize");
    session
        .write_str("printf 'STTY2:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("write");
    let second = wait_all(&mut session, &["\nSTTY2:", ":END"]);
    let after = extract_marked(&second, "STTY2:", ":END");
    assert_eq!(before, after);
}

fn extract_marked(output: &[u8], prefix: &str, suffix: &str) -> String {
    let text = String::from_utf8_lossy(output);
    text.rfind(prefix)
        .and_then(|start| {
            let rest = &text[start + prefix.len()..];
            rest.find(suffix).map(|end| rest[..end].to_string())
        })
        .unwrap_or_else(|| panic!("missing {prefix}..{suffix} in {text}"))
}
