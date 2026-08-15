use mbx_pty::{
    DEFAULT_CAPTURE_LIMIT, PtyError, PtySession, SpawnOptions, WinSize, visible_contains,
    visible_text,
};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn deadline(seconds: u64) -> Instant {
    Instant::now() + Duration::from_secs(seconds)
}

fn path_env() -> OsString {
    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
}

struct TempHome(PathBuf);

impl TempHome {
    fn new() -> Self {
        let dir = env::temp_dir().join(format!("mbx-hist-{}-{}", std::process::id(), {
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

    fn histfile(&self) -> PathBuf {
        self.0.join(".bash_history")
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn spawn_history_bash(
    home: &Path,
    histfile: &Path,
    histcontrol: Option<&str>,
    histignore: Option<&str>,
) -> PtySession {
    let mut options = SpawnOptions::new("/bin/bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-i")
        .clear_env()
        .env("PATH", path_env())
        .env("HOME", home)
        .env("TERM", "xterm")
        .env("HISTFILE", histfile)
        .env("HISTSIZE", "1000")
        .env("HISTFILESIZE", "1000")
        .env("PS1", "HIST# ")
        .env("PS2", "CONT> ")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .cwd(home)
        .winsize(WinSize { rows: 24, cols: 80 });
    if let Some(value) = histcontrol {
        options = options.env("HISTCONTROL", value);
    }
    if let Some(value) = histignore {
        options = options.env("HISTIGNORE", value);
    }
    PtySession::spawn(options).expect("history bash spawn")
}

fn write_dump_script(home: &Path) -> PathBuf {
    let script = home.join("dump.bash");
    fs::write(&script, "history -a\nprintf 'MBX_DUMP_END\\n'\n").expect("dump script");
    script
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

fn lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect()
}

fn dump_history(session: &mut PtySession, home: &Path, histfile: &Path) -> String {
    let script = write_dump_script(home);
    type_line(session, &format!("source \"{}\"", script.display()));
    wait_for(session, "MBX_DUMP_END");
    fs::read_to_string(histfile).expect("history file")
}

fn exit_and_wait(session: &mut PtySession) {
    type_line(session, "exit");
    match session.read_until(deadline(8), DEFAULT_CAPTURE_LIMIT, |_| false) {
        Err(PtyError::ChildExited) => {}
        Ok(_) => panic!("shell did not exit"),
        Err(error) => panic!("shell exit failed: {error}"),
    }
    session
        .wait()
        .expect("wait")
        .success()
        .then_some(())
        .expect("clean exit");
}

#[test]
fn simple_commands_are_admitted_to_history() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "echo alpha");
    wait_for(&mut session, "\nalpha");
    type_line(&mut session, "echo beta");
    wait_for(&mut session, "\nbeta");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("echo alpha"), "dump={dump:?}");
    assert!(dump.contains("echo beta"), "dump={dump:?}");
}

#[test]
fn ignorespace_skips_leading_space_entries() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), Some("ignorespace"), None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, " echo hidden");
    type_line(&mut session, "echo visible");
    wait_for(&mut session, "\nvisible");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("echo visible"), "dump={dump:?}");
    assert!(
        !dump.lines().any(|line| line == " echo hidden"),
        "dump={dump:?}"
    );
}

#[test]
fn ignoredups_keeps_one_consecutive_duplicate() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), Some("ignoredups"), None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "echo dup");
    wait_for(&mut session, "\ndup");
    type_line(&mut session, "echo dup");
    wait_for(&mut session, "\ndup");
    type_line(&mut session, "echo other");
    wait_for(&mut session, "\nother");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    let count = dump.lines().filter(|line| *line == "echo dup").count();
    assert_eq!(count, 1, "dump={dump:?}");
}

#[test]
fn ignoreboth_applies_space_and_duplicate_rules() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), Some("ignoreboth"), None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, " echo hidden");
    type_line(&mut session, "echo dup");
    wait_for(&mut session, "\ndup");
    type_line(&mut session, "echo dup");
    wait_for(&mut session, "\ndup");
    type_line(&mut session, "echo keep");
    wait_for(&mut session, "\nkeep");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("echo keep"), "dump={dump:?}");
    assert!(
        !dump.lines().any(|line| line == " echo hidden"),
        "dump={dump:?}"
    );
    assert_eq!(
        dump.lines().filter(|line| *line == "echo dup").count(),
        1,
        "dump={dump:?}"
    );
}

#[test]
fn erasedups_removes_earlier_occurrences() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), Some("erasedups"), None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "echo same");
    wait_for(&mut session, "\nsame");
    type_line(&mut session, "echo mid");
    wait_for(&mut session, "\nmid");
    type_line(&mut session, "echo same");
    wait_for(&mut session, "\nsame");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    let file_lines = lines(&dump);
    assert_eq!(
        file_lines
            .iter()
            .filter(|line| **line == "echo same")
            .count(),
        1,
        "dump={dump:?}"
    );
    let mid = file_lines.iter().position(|line| *line == "echo mid");
    let same = file_lines.iter().position(|line| *line == "echo same");
    assert!(
        mid.is_some() && same.is_some() && mid < same,
        "dump={dump:?}"
    );
}

#[test]
fn histignore_excludes_matching_commands() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), None, Some("rm *"));
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "rm temp.txt");
    type_line(&mut session, "echo keep");
    wait_for(&mut session, "\nkeep");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("echo keep"), "dump={dump:?}");
    assert!(!dump.contains("rm temp.txt"), "dump={dump:?}");
}

#[test]
fn history_off_suppresses_admission() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "set +o history");
    type_line(&mut session, "echo hidden");
    wait_for(&mut session, "\nhidden");
    type_line(&mut session, "set -o history");
    type_line(&mut session, "echo visible");
    wait_all(&mut session, &["\nvisible", "HIST# "]);
    type_line(&mut session, "printf 'HISTOFF:%s\\n' \"$HISTCMD\"");
    let output = wait_all(&mut session, &["\nHISTOFF:3", "HIST# "]);
    let text = visible_text(&output);
    assert!(
        text.lines().any(|line| line.trim() == "HISTOFF:3"),
        "HISTCMD must not advance while history is disabled; output={text:?}"
    );
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("echo visible"), "dump={dump:?}");
    assert!(!dump.contains("echo hidden"), "dump={dump:?}");
    assert!(!dump.contains("set -o history"), "dump={dump:?}");
}

#[test]
fn history_dash_s_injects_without_executing() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "history -s 'injected-marker'");
    type_line(&mut session, "echo real-output");
    let output = wait_all(&mut session, &["\nreal-output", "HIST# "]);
    let text = visible_text(&output);
    assert!(
        !lines(&text).contains(&"injected-marker"),
        "injected command must not execute; output={text:?}"
    );
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("injected-marker"), "dump={dump:?}");
    assert!(dump.contains("echo real-output"), "dump={dump:?}");
}

#[test]
fn multiline_command_is_stored_as_one_entry() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), None, None);
    wait_for(&mut session, "HIST# ");
    session
        .write_str("echo one \\\n", deadline(2))
        .expect("write");
    wait_for(&mut session, "CONT> ");
    session.write_str("two\n", deadline(2)).expect("write");
    wait_for(&mut session, "\none two");
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(
        dump.contains("echo one two"),
        "Bash folds the continuation into a single space-joined entry; dump={dump:?}"
    );
}

#[test]
fn history_d_renumbers_but_histcmd_stays_monotonic() {
    let home = TempHome::new();
    let mut session = spawn_history_bash(home.path(), &home.histfile(), None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "echo a");
    wait_for(&mut session, "\na");
    type_line(&mut session, "echo b");
    wait_for(&mut session, "\nb");
    type_line(&mut session, "echo c");
    wait_for(&mut session, "\nc");
    type_line(&mut session, "history -d 2");
    type_line(&mut session, "echo NEXT:$HISTCMD");
    let output = wait_all(&mut session, &["\nNEXT:", "HIST# "]);
    let text = visible_text(&output);
    assert!(
        text.lines().any(|line| line.trim().starts_with("NEXT:4")),
        "HISTCMD must stay monotonic past the last entry; output={text:?}"
    );
    let dump = dump_history(&mut session, home.path(), &home.histfile());
    assert!(dump.contains("echo a"), "dump={dump:?}");
    assert!(
        !dump.contains("echo b"),
        "deleted entry must be gone: {dump:?}"
    );
    assert!(dump.contains("echo c"), "dump={dump:?}");
}

#[test]
fn history_a_appends_without_rewriting_prior_entries() {
    let home = TempHome::new();
    let histfile = home.histfile();
    fs::write(&histfile, "echo prior\n").expect("seed history");
    let mut session = spawn_history_bash(home.path(), &histfile, None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "echo current");
    wait_all(&mut session, &["\ncurrent", "HIST# "]);
    let content = dump_history(&mut session, home.path(), &histfile);
    assert_eq!(
        content.lines().filter(|line| *line == "echo prior").count(),
        1,
        "history -a must preserve prior entries without duplicating them; content={content:?}"
    );
    assert!(content.contains("echo current"), "content={content:?}");
}

#[test]
fn exit_flush_writes_the_session_list() {
    let home = TempHome::new();
    let histfile = home.histfile();
    fs::write(&histfile, "echo preexisting\n").expect("seed history");
    let mut session = spawn_history_bash(home.path(), &histfile, None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "echo session-cmd");
    wait_for(&mut session, "\nsession-cmd");
    exit_and_wait(&mut session);
    let content = fs::read_to_string(&histfile).expect("history file");
    assert!(content.contains("echo session-cmd"), "content={content:?}");
    assert!(content.contains("echo preexisting"), "content={content:?}");
}

#[test]
fn exit_flush_with_histappend_preserves_prior_entries() {
    let home = TempHome::new();
    let histfile = home.histfile();
    fs::write(&histfile, "echo preexisting\n").expect("seed history");
    let mut session = spawn_history_bash(home.path(), &histfile, None, None);
    wait_for(&mut session, "HIST# ");
    type_line(&mut session, "shopt -s histappend");
    type_line(&mut session, "echo session-cmd");
    wait_for(&mut session, "\nsession-cmd");
    exit_and_wait(&mut session);
    let content = fs::read_to_string(&histfile).expect("history file");
    assert!(content.contains("echo session-cmd"), "content={content:?}");
    assert!(content.contains("echo preexisting"), "content={content:?}");
    assert_eq!(
        content
            .lines()
            .filter(|line| *line == "echo preexisting")
            .count(),
        1,
        "histappend must not duplicate the seeded entry; content={content:?}"
    );
}

#[test]
fn noninteractive_shell_does_not_write_history() {
    let home = TempHome::new();
    let histfile = home.histfile();
    let status = Command::new("/bin/bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-c")
        .arg("echo x")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env("HOME", home.path())
        .env("HISTFILE", &histfile)
        .status()
        .expect("noninteractive bash");
    assert!(status.success());
    assert!(
        !histfile.exists(),
        "noninteractive shell must not create HISTFILE"
    );
}
