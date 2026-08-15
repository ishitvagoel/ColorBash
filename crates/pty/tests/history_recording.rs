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
        panic!("mbx binary missing; build the workspace before history PTY tests");
    }
}

fn path_env() -> OsString {
    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
}

struct TempHome(PathBuf);

impl TempHome {
    fn new() -> Self {
        let dir = env::temp_dir().join(format!("mbx-hrec-{}-{}", std::process::id(), {
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

fn spawn_history_shell(home: &Path, extra_env: &[(&str, &str)]) -> PtySession {
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
        .env("TERM", "xterm")
        .env("USER", "mbx")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("MBX_TEST_ROOT", workspace_root())
        .env("MBX_BIN", mbx_bin())
        .env("MBX_COLOR", "never")
        .env("MBX_ICONS", "never")
        .env("MBX_DISABLE_GIT", "1")
        // These PTY cases assert history semantics, not the production 100 ms
        // transport budget. Keep their exchanges bounded but tolerant of
        // heavily parallel CI load; focused Bash tests cover deadline behavior.
        .env("MBX_IPC_TIMEOUT", "1.0")
        .env("MBX_HISTORY_TIMEOUT", "1.0")
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 });
    for &(key, value) in extra_env {
        options = options.env(key, value);
    }
    PtySession::spawn(options).expect("history shell spawn")
}

fn wait_for(session: &mut PtySession, needle: &str) -> Vec<u8> {
    wait_all(session, &[needle])
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
            match &error {
                PtyError::Timeout(captured) => visible_text(captured),
                _ => error.to_string(),
            }
        ),
    }
}

fn type_line(session: &mut PtySession, line: &str) {
    session
        .write_str(&format!("{line}\n"), deadline(2))
        .expect("write");
}

fn query(bin: &Path, args: &[&str], data_home: &Path) -> String {
    let output = Command::new(bin)
        .args(args)
        .env("MBX_HISTORY", "1")
        .env("XDG_DATA_HOME", data_home)
        .env("HOME", data_home.parent().unwrap_or(data_home))
        .output()
        .expect("mbx history query");
    assert!(
        output.status.success(),
        "mbx {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn count_entries(bin: &Path, data_home: &Path) -> u64 {
    query(bin, &["history", "count"], data_home)
        .trim()
        .parse()
        .expect("count is numeric")
}

fn wait_for_count(bin: &Path, data_home: &Path, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let current = count_entries(bin, data_home);
        if current == expected {
            return;
        }
        if Instant::now() >= deadline {
            let recent = query(bin, &["history", "search", "recent"], data_home);
            let store_dir = data_home.join("mbx");
            let files: Vec<String> = fs::read_dir(&store_dir)
                .map(|entries| {
                    entries
                        .filter_map(|entry| entry.ok())
                        .map(|entry| entry.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            panic!(
                "history count never reached {expected}; last={current} recent={recent:?} files={files:?}"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn admitted_commands_are_recorded_through_mbx2() {
    let home = TempHome::new();
    let data_home = home.path().join("data");
    fs::create_dir_all(&data_home).expect("data home");
    let mut session = spawn_history_shell(
        home.path(),
        &[
            ("MBX_HISTORY", "1"),
            ("XDG_DATA_HOME", data_home.to_str().unwrap()),
            (
                "HISTFILE",
                home.path().join(".bash_history").to_str().unwrap(),
            ),
        ],
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo alpha");
    wait_all(&mut session, &["\nalpha", "> "]);
    type_line(&mut session, "echo beta");
    wait_all(&mut session, &["\nbeta", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    type_line(&mut session, "exit");
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |_| false) {
        Err(PtyError::ChildExited) => {}
        Ok(_) => panic!("shell did not exit"),
        Err(error) => panic!("shell exit failed: {error}"),
    }
    session.wait().expect("wait");

    let recent = query(&mbx_bin(), &["history", "search", "recent"], &data_home);
    assert!(recent.contains("echo beta"), "recent={recent:?}");
    assert!(recent.contains("echo alpha"), "recent={recent:?}");
    let prefix = query(
        &mbx_bin(),
        &["history", "search", "prefix", "echo"],
        &data_home,
    );
    assert!(prefix.contains("echo alpha"), "prefix={prefix:?}");
    let by_cwd = query(
        &mbx_bin(),
        &["history", "search", "cwd", home.path().to_str().unwrap()],
        &data_home,
    );
    assert!(by_cwd.contains("echo alpha"), "cwd={by_cwd:?}");
}

#[test]
fn excluded_commands_are_not_recorded() {
    let home = TempHome::new();
    let data_home = home.path().join("data");
    fs::create_dir_all(&data_home).expect("data home");
    let mut session = spawn_history_shell(
        home.path(),
        &[
            ("MBX_HISTORY", "1"),
            ("MBX_HISTORY_EXCLUDE", "git *"),
            ("XDG_DATA_HOME", data_home.to_str().unwrap()),
            (
                "HISTFILE",
                home.path().join(".bash_history").to_str().unwrap(),
            ),
        ],
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "git status");
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo keep");
    wait_all(&mut session, &["\nkeep", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    type_line(&mut session, "exit");
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |_| false) {
        Err(PtyError::ChildExited) => {}
        Ok(_) => panic!("shell did not exit"),
        Err(error) => panic!("shell exit failed: {error}"),
    }
    session.wait().expect("wait");

    let prefix = query(
        &mbx_bin(),
        &["history", "search", "prefix", "echo"],
        &data_home,
    );
    assert!(prefix.contains("echo keep"), "prefix={prefix:?}");
    let git = query(
        &mbx_bin(),
        &["history", "search", "prefix", "git"],
        &data_home,
    );
    assert!(
        git.trim().is_empty(),
        "git commands must be excluded: {git:?}"
    );
}

#[test]
fn history_is_disabled_by_default_and_creates_no_store() {
    let home = TempHome::new();
    let data_home = home.path().join("data");
    fs::create_dir_all(&data_home).expect("data home");
    let mut session = spawn_history_shell(
        home.path(),
        &[
            ("XDG_DATA_HOME", data_home.to_str().unwrap()),
            (
                "HISTFILE",
                home.path().join(".bash_history").to_str().unwrap(),
            ),
        ],
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo hidden");
    wait_all(&mut session, &["\nhidden", "> "]);
    type_line(&mut session, "exit");
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |_| false) {
        Err(PtyError::ChildExited) => {}
        Ok(_) => panic!("shell did not exit"),
        Err(error) => panic!("shell exit failed: {error}"),
    }
    session.wait().expect("wait");

    assert!(
        !data_home.join("mbx/history.sqlite3").exists(),
        "disabled history must not create a store"
    );
}

#[test]
fn hostile_command_text_remains_inert() {
    let home = TempHome::new();
    let data_home = home.path().join("data");
    fs::create_dir_all(&data_home).expect("data home");
    let mut session = spawn_history_shell(
        home.path(),
        &[
            ("MBX_HISTORY", "1"),
            ("XDG_DATA_HOME", data_home.to_str().unwrap()),
            (
                "HISTFILE",
                home.path().join(".bash_history").to_str().unwrap(),
            ),
        ],
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo 'a%20b'");
    wait_all(&mut session, &["\na%20b", "> "]);
    type_line(&mut session, "printf 'x-tab-\\t-\\n'");
    wait_all(&mut session, &["\nx-tab-", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    type_line(&mut session, "exit");
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |_| false) {
        Err(PtyError::ChildExited) => {}
        Ok(_) => panic!("shell did not exit"),
        Err(error) => panic!("shell exit failed: {error}"),
    }
    session.wait().expect("wait");

    let recent = query(&mbx_bin(), &["history", "search", "recent"], &data_home);
    assert!(recent.contains("printf"), "recent={recent:?}");
}
