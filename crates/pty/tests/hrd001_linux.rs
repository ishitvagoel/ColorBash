//! HRD-001 Linux pairwise PTY (L-1–L-5). Does not close macOS or G5.
use mbx_pty::{
    DEFAULT_CAPTURE_LIMIT, PtyError, PtySession, SpawnOptions, WinSize, visible_contains,
    visible_text,
};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
        panic!("mbx binary missing; build the workspace before HRD-001 Linux PTY tests");
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

fn spawn_options(bin: &Path, home: &Path) -> SpawnOptions {
    write_rc(home);
    SpawnOptions::new("/bin/bash")
        .arg("--noprofile")
        .arg("--rcfile")
        .arg(home.join("rc.bash"))
        .arg("-i")
        .clear_env()
        .env("PATH", path_env())
        .env("HOME", home)
        .env("TERM", "xterm")
        .env("USER", "mbx")
        .env("HOSTNAME", "testhost")
        .env("HISTFILE", "/dev/null")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("MBX_TEST_ROOT", workspace_root())
        .env("MBX_BIN", bin)
        .env("MBX_COLOR", "never")
        .env("MBX_ICONS", "never")
        .env("MBX_DISABLE_GIT", "1")
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 })
}

fn spawn_mbx(bin: &Path, home: &Path) -> PtySession {
    PtySession::spawn(spawn_options(bin, home)).expect("MBX PTY spawn")
}

fn wait_for(session: &mut PtySession, needle: &str) -> Vec<u8> {
    match session.read_until(deadline(12), DEFAULT_CAPTURE_LIMIT, |output| {
        visible_contains(output, needle)
    }) {
        Ok(output) => output,
        Err(error) => panic!(
            "waiting for {needle:?} failed: {error} output={:?}",
            captured_preview(&error)
        ),
    }
}

fn wait_all(session: &mut PtySession, needles: &[&str]) -> Vec<u8> {
    match session.read_until(deadline(12), DEFAULT_CAPTURE_LIMIT, |output| {
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

fn captured_preview(error: &PtyError) -> String {
    match error {
        PtyError::Timeout(captured) => visible_text(captured),
        _ => error.to_string(),
    }
}

fn wait_prompt(session: &mut PtySession) -> Vec<u8> {
    wait_for(session, "> ")
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

struct TempHome(PathBuf);

impl TempHome {
    fn new() -> Self {
        let dir = env::temp_dir().join(format!("mbx-hrd001-{}-{}", std::process::id(), {
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
fn nested_interactive_bash_returns_to_mbx_prompt() {
    let home = TempHome::new();
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    session
        .write_str("bash --noprofile --norc -i\n", deadline(2))
        .expect("write inner bash");
    session
        .write_str("printf 'MBX_HRD:nested\\n'\n", deadline(2))
        .expect("write nested marker");
    wait_for(&mut session, "\nMBX_HRD:nested");
    session
        .write_str("exit\n", deadline(2))
        .expect("exit inner");
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_HRD:outer\\n'\n", deadline(2))
        .expect("write outer marker");
    wait_for(&mut session, "\nMBX_HRD:outer");
}

#[test]
fn ssh_connection_shows_ssh_host_in_the_live_prompt() {
    let home = TempHome::new();
    let mut session = PtySession::spawn(
        spawn_options(&mbx_bin(), home.path()).env("SSH_CONNECTION", "10.0.0.1 1234 10.0.0.2 22"),
    )
    .expect("SSH PTY spawn");
    wait_all(&mut session, &["ssh: testhost", "> "]);
    session
        .write_str("printf 'MBX_HRD:ssh\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_HRD:ssh");
}

#[test]
fn login_shell_sources_profile_and_stays_usable() {
    let home = TempHome::new();
    fs::write(
        home.path().join(".bash_profile"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("bash_profile");
    let mut session = PtySession::spawn(
        SpawnOptions::new("/bin/bash")
            .arg("-il")
            .clear_env()
            .env("PATH", path_env())
            .env("HOME", home.path())
            .env("TERM", "xterm")
            .env("USER", "mbx")
            .env("HOSTNAME", "testhost")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", mbx_bin())
            .env("MBX_COLOR", "never")
            .env("MBX_ICONS", "never")
            .env("MBX_DISABLE_GIT", "1")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("login PTY spawn");
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_HRD:login\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_HRD:login");
}

#[test]
fn vim_fullscreen_quits_and_restores_stty() {
    let home = TempHome::new();
    let file = home.path().join("MBX_HRD_VIM.txt");
    fs::write(&file, "hello-vim\n").expect("vim file");
    let mut session = spawn_mbx(&mbx_bin(), home.path());
    wait_prompt(&mut session);
    session
        .write_str("printf 'STTY1:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("stty before");
    let first = wait_all(&mut session, &["\nSTTY1:", ":END"]);
    let before = extract_marked(&first, "STTY1:", ":END");
    session
        .write_str(
            "vim -u NONE -n -c qa MBX_HRD_VIM.txt; printf 'MBX_HRD:after_vim\\n'\n",
            deadline(2),
        )
        .expect("run vim then marker");
    wait_for(&mut session, "\nMBX_HRD:after_vim");
    session
        .write_str("printf 'STTY2:%s:END\\n' \"$(stty -g)\"\n", deadline(2))
        .expect("stty after");
    let second = wait_all(&mut session, &["\nSTTY2:", ":END"]);
    let after = extract_marked(&second, "STTY2:", ":END");
    assert_eq!(before, after, "vim must restore terminal settings");
    session
        .write_str("printf 'MBX_HRD:after_vim\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_HRD:after_vim");
}

#[test]
fn tmux_session_runs_mbx_prompt_and_a_command() {
    let tmux = Path::new("/usr/bin/tmux");
    assert!(
        tmux.is_file(),
        "HRD-001 L-5 needs /usr/bin/tmux on this Linux host"
    );
    let home = TempHome::new();
    write_rc(home.path());
    let socket = home.path().join("tmux.sock");
    fs::write(
        home.path().join("tmux.conf"),
        "set -g status off\nset -g default-terminal xterm\n",
    )
    .expect("tmux.conf");
    let mut session = PtySession::spawn(
        SpawnOptions::new(tmux)
            .arg("-f")
            .arg(home.path().join("tmux.conf"))
            .arg("-S")
            .arg(&socket)
            .arg("-u")
            .arg("new-session")
            .arg("-x")
            .arg("80")
            .arg("-y")
            .arg("24")
            .arg("--")
            .arg("/bin/bash")
            .arg("--noprofile")
            .arg("--rcfile")
            .arg(home.path().join("rc.bash"))
            .arg("-i")
            .clear_env()
            .env("PATH", path_env())
            .env("HOME", home.path())
            .env("TERM", "xterm")
            .env("USER", "mbx")
            .env("HOSTNAME", "testhost")
            .env("HISTFILE", "/dev/null")
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("MBX_TEST_ROOT", workspace_root())
            .env("MBX_BIN", mbx_bin())
            .env("MBX_COLOR", "never")
            .env("MBX_ICONS", "never")
            .env("MBX_DISABLE_GIT", "1")
            .cwd(home.path())
            .winsize(WinSize { rows: 24, cols: 80 }),
    )
    .expect("tmux PTY spawn");
    wait_prompt(&mut session);
    session
        .write_str("printf 'MBX_HRD:tmux\\n'\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "\nMBX_HRD:tmux");
    drop(session);
    let status = Command::new(tmux)
        .arg("-S")
        .arg(&socket)
        .arg("has-session")
        .status();
    match status {
        Ok(code) if !code.success() => {}
        Ok(_) => {
            let _ = Command::new(tmux)
                .arg("-S")
                .arg(&socket)
                .arg("kill-server")
                .status();
        }
        Err(_) => {}
    }
}
