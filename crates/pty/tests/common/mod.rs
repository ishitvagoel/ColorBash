#![allow(dead_code)]

use mbx_pty::{
    DEFAULT_CAPTURE_LIMIT, PtyError, PtySession, SpawnOptions, WinSize, visible_contains,
    visible_text,
};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

pub fn deadline(seconds: u64) -> Instant {
    Instant::now() + Duration::from_secs(seconds)
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

pub fn mbx_bin() -> PathBuf {
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

pub fn path_env() -> OsString {
    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
}

pub struct TempHome(PathBuf);

impl TempHome {
    pub fn new(prefix: &str) -> Self {
        let dir = env::temp_dir().join(format!("mbx-{prefix}-{}-{}", std::process::id(), {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        }));
        fs::create_dir_all(&dir).expect("temp home");
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn histfile(&self) -> PathBuf {
        self.0.join(".bash_history")
    }

    pub fn data_home(&self) -> PathBuf {
        let path = self.0.join("data");
        fs::create_dir_all(&path).expect("data home");
        path
    }

    pub fn store_path(&self) -> PathBuf {
        self.data_home().join("mbx/history.sqlite3")
    }

    pub fn ack_samples_path(&self) -> PathBuf {
        self.data_home().join("mbx/history-ack-samples")
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn spawn_history_shell(home: &Path, extra_env: &[(&str, &str)]) -> PtySession {
    spawn_history_shell_with_timeouts(home, extra_env, "1.0", "1.0")
}

pub fn spawn_history_shell_production_timeouts(
    home: &Path,
    extra_env: &[(&str, &str)],
) -> PtySession {
    spawn_history_shell_with_timeouts(home, extra_env, "0.10", "0.10")
}

pub fn spawn_history_shell_with_timeouts(
    home: &Path,
    extra_env: &[(&str, &str)],
    ipc_timeout: &str,
    history_timeout: &str,
) -> PtySession {
    spawn_history_shell_rc_with_timeouts(home, extra_env, "", ipc_timeout, history_timeout)
}

pub fn spawn_history_shell_rc(
    home: &Path,
    extra_env: &[(&str, &str)],
    rc_prelude: &str,
) -> PtySession {
    spawn_history_shell_rc_with_timeouts(home, extra_env, rc_prelude, "1.0", "1.0")
}

fn spawn_history_shell_rc_with_timeouts(
    home: &Path,
    extra_env: &[(&str, &str)],
    rc_prelude: &str,
    ipc_timeout: &str,
    history_timeout: &str,
) -> PtySession {
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
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("HISTSIZE", "1000")
        .env("HISTFILESIZE", "1000")
        .env("MBX_TEST_ROOT", workspace_root())
        .env("MBX_BIN", mbx_bin())
        .env("MBX_COLOR", "never")
        .env("MBX_ICONS", "never")
        .env("MBX_DISABLE_GIT", "1")
        // These PTY cases assert history semantics, not the production 100 ms
        // transport budget. Keep their exchanges bounded but tolerant of
        // heavily parallel CI load; focused Bash tests cover deadline behavior.
        .env("MBX_IPC_TIMEOUT", ipc_timeout)
        .env("MBX_HISTORY_TIMEOUT", history_timeout)
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 });
    for &(key, value) in extra_env {
        options = options.env(key, value);
    }
    PtySession::spawn(options).expect("history shell spawn")
}

pub fn wait_for(session: &mut PtySession, needle: &str) -> Vec<u8> {
    wait_all(session, &[needle])
}

pub fn wait_all(session: &mut PtySession, needles: &[&str]) -> Vec<u8> {
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

pub fn wait_prompt_after_ctrl_c(session: &mut PtySession) -> Vec<u8> {
    // The live editing line is `> typed-text`. Matching `> ` or `^C` on that
    // line is not a new prompt. A completed interrupt redraws the context
    // line (`exit 130`) and then a bare input anchor (`\n> `).
    let new_prompt_after_interrupt = |output: &[u8]| {
        let text = visible_text(output);
        if let Some(idx) = text.find("exit 130") {
            return text[idx..].contains("\n> ");
        }
        text.find("^C")
            .is_some_and(|idx| text[idx..].contains("\n> "))
    };
    match session.read_until(
        deadline(2),
        DEFAULT_CAPTURE_LIMIT,
        new_prompt_after_interrupt,
    ) {
        Ok(output) => output,
        Err(PtyError::Timeout(captured)) => {
            session
                .write_all(&[0x03], deadline(2))
                .expect("second ctrl-c");
            match session.read_until(
                deadline(8),
                DEFAULT_CAPTURE_LIMIT,
                new_prompt_after_interrupt,
            ) {
                Ok(rest) => {
                    let mut all = captured;
                    all.extend(rest);
                    all
                }
                Err(error) => panic!(
                    "waiting for interrupt prompt failed: {error} first={:?} output={:?}",
                    visible_text(&captured),
                    match &error {
                        PtyError::Timeout(more) => visible_text(more),
                        _ => error.to_string(),
                    }
                ),
            }
        }
        Err(error) => panic!(
            "waiting for interrupt prompt failed: {error} output={:?}",
            error.to_string()
        ),
    }
}

pub fn kill_line(session: &mut PtySession) {
    session
        .write_all(&[0x15], deadline(2))
        .expect("ctrl-u kill line");
}

pub fn type_line(session: &mut PtySession, line: &str) {
    session
        .write_str(&format!("{line}\n"), deadline(2))
        .expect("write");
}

pub fn query(bin: &Path, args: &[&str], data_home: &Path) -> String {
    let output = run_history(bin, args, data_home);
    assert!(
        output.status.success(),
        "mbx {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn run_history(bin: &Path, args: &[&str], data_home: &Path) -> Output {
    Command::new(bin)
        .args(args)
        .env("MBX_HISTORY", "1")
        .env("XDG_DATA_HOME", data_home)
        .env("HOME", data_home.parent().unwrap_or(data_home))
        .output()
        .expect("mbx history command")
}

pub fn count_entries(bin: &Path, data_home: &Path) -> u64 {
    query(bin, &["history", "count"], data_home)
        .trim()
        .parse()
        .expect("count is numeric")
}

pub fn wait_for_count(bin: &Path, data_home: &Path, expected: u64) {
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

pub fn sidecar_commands(bin: &Path, data_home: &Path) -> Vec<String> {
    query(
        bin,
        &["history", "search", "recent", "--limit", "50"],
        data_home,
    )
    .lines()
    .map(str::to_owned)
    .filter(|line| !line.is_empty())
    .collect()
}

pub fn dump_histfile(session: &mut PtySession, home: &Path, histfile: &Path) -> String {
    let script = home.join("dump.bash");
    fs::write(&script, "history -a\nprintf 'MBX_DUMP_END\\n'\n").expect("dump script");
    type_line(session, "source dump.bash");
    wait_all(session, &["MBX_DUMP_END", "> "]);
    fs::read_to_string(histfile).unwrap_or_default()
}

pub fn histfile_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim_end_matches('\r').to_owned())
        .collect()
}

pub fn exit_and_wait(session: &mut PtySession) {
    type_line(session, "exit");
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |_| false) {
        Err(PtyError::ChildExited) => {}
        Ok(_) => panic!("shell did not exit"),
        Err(error) => panic!("shell exit failed: {error}"),
    }
    session.wait().expect("wait");
}

pub fn enabled_env<'a>(
    data_home: &'a str,
    histfile: &'a str,
    extra: &'a [(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut env = vec![
        ("MBX_HISTORY", "1"),
        ("XDG_DATA_HOME", data_home),
        ("HISTFILE", histfile),
    ];
    env.extend_from_slice(extra);
    env
}

pub fn disabled_env<'a>(
    data_home: &'a str,
    histfile: &'a str,
    extra: &'a [(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut env = vec![("XDG_DATA_HOME", data_home), ("HISTFILE", histfile)];
    env.extend_from_slice(extra);
    env
}

pub fn read_ack_samples(path: &Path) -> Vec<u64> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.parse::<u64>()
                .unwrap_or_else(|_| panic!("sample line is not an unsigned integer: {line:?}"))
        })
        .collect()
}

pub fn assert_ack_samples_digits_only(path: &Path, forbidden: &str) {
    let text = fs::read_to_string(path).expect("ack sample file must exist");
    assert!(
        !text.contains(forbidden),
        "ack sample file must not contain forbidden text"
    );
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        assert!(
            line.chars().all(|character| character.is_ascii_digit()),
            "ack sample line must be digits only: {line:?}"
        );
    }
}
