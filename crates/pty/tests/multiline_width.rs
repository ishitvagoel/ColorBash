use mbx_pty::{
    DEFAULT_CAPTURE_LIMIT, PtyError, PtySession, SpawnOptions, WinSize, visible_contains,
    visible_text,
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
        panic!("mbx binary missing; build the workspace before PTY width tests");
    }
}

fn path_env() -> OsString {
    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
}

struct TempHome(PathBuf);

impl TempHome {
    fn new() -> Self {
        let dir = env::temp_dir().join(format!("mbx-width-{}-{}", std::process::id(), {
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

fn spawn_mbx(home: &Path, winsize: WinSize) -> PtySession {
    fs::write(
        home.join("rc.bash"),
        "source \"${MBX_TEST_ROOT}/bash/init.bash\"\n",
    )
    .expect("rcfile");
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
        .env("MBX_BIN", mbx_bin())
        .env("MBX_COLOR", "never")
        .env("MBX_ICONS", "never")
        .env("MBX_DISABLE_GIT", "1")
        .env("PS2", "CONT> ")
        .cwd(home)
        .winsize(winsize);
    PtySession::spawn(options).expect("MBX width spawn")
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

#[test]
fn two_line_prompt_renders_and_accepts_input() {
    let home = TempHome::new();
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 80 });
    wait_for(&mut session, "> ");
    type_line(&mut session, "printf 'MBX_WIDTH:ok\\n'");
    wait_all(&mut session, &["\nMBX_WIDTH:ok", "> "]);
}

#[test]
fn multiline_input_uses_ps2_continuation() {
    let home = TempHome::new();
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 80 });
    wait_for(&mut session, "> ");
    session
        .write_str("echo one \\\n", deadline(2))
        .expect("write");
    let continuation = wait_for(&mut session, "CONT> ");
    assert!(
        visible_contains(&continuation, "echo one \\\nCONT> "),
        "expected the typed continuation and PS2 prompt; output={:?}",
        visible_text(&continuation)
    );
    session.write_str("two\n", deadline(2)).expect("write");
    wait_all(&mut session, &["\none two", "> "]);
}

#[test]
fn narrow_prompt_wraps_without_breaking_input() {
    let home = TempHome::new();
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 20 });
    wait_for(&mut session, "> ");
    type_line(&mut session, "printf 'MBX_WIDTH:narrow-ok\\n'");
    wait_all(&mut session, &["\nMBX_WIDTH:narrow-ok", "> "]);
}

#[test]
fn resize_mid_line_preserves_the_buffer() {
    let home = TempHome::new();
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 80 });
    wait_for(&mut session, "> ");
    session
        .write_str("printf 'MBX_WIDTH:resized-", deadline(2))
        .expect("partial write");
    session
        .resize(WinSize { rows: 10, cols: 40 })
        .expect("resize");
    session
        .write_str("ok\\n'\n", deadline(2))
        .expect("complete write");
    wait_all(&mut session, &["\nMBX_WIDTH:resized-ok", "> "]);
}

#[test]
fn wide_glyph_directory_renders_and_accepts_input() {
    let home = TempHome::new();
    let wide = home.path().join("测 试 目录");
    fs::create_dir_all(&wide).expect("wide dir");
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 80 });
    wait_for(&mut session, "> ");
    type_line(&mut session, "cd \"测 试 目录\"");
    wait_for(&mut session, "> ");
    type_line(&mut session, "pwd");
    wait_all(&mut session, &["/测 试 目录", "> "]);
    type_line(&mut session, "printf 'MBX_WIDTH:unicode-ok\\n'");
    wait_all(&mut session, &["\nMBX_WIDTH:unicode-ok", "> "]);
}

#[test]
fn long_single_line_wraps_without_corrupting_the_buffer() {
    let home = TempHome::new();
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 30 });
    wait_for(&mut session, "> ");
    let long =
        "printf 'MBX_WIDTH:long-ok\\n'; printf '1234567890123456789012345678901234567890\\n'";
    type_line(&mut session, long);
    wait_all(
        &mut session,
        &[
            "\nMBX_WIDTH:long-ok",
            "\n1234567890123456789012345678901234567890",
            "> ",
        ],
    );
}

#[test]
fn combining_mark_path_keeps_input_usable() {
    let home = TempHome::new();
    let combined = home.path().join("e\u{301}tude");
    fs::create_dir_all(&combined).expect("combining dir");
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 80 });
    wait_for(&mut session, "> ");
    type_line(&mut session, "cd \"e\u{301}tude\"");
    wait_for(&mut session, "> ");
    type_line(&mut session, "printf 'MBX_WIDTH:combining-ok\\n'");
    wait_all(&mut session, &["\nMBX_WIDTH:combining-ok", "> "]);
}

#[test]
fn narrow_wrap_long_command_stays_usable() {
    let home = TempHome::new();
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 20 });
    wait_for(&mut session, "> ");
    type_line(
        &mut session,
        "printf 'MBX_WRAP:narrow-long-ok\\n'; echo abcdefghijklmnopqrstuvwxyz",
    );
    wait_all(
        &mut session,
        &[
            "\nMBX_WRAP:narrow-long-ok",
            "\nabcdefghijklmnopqrstuvwxyz",
            "> ",
        ],
    );
}

#[test]
fn narrow_wrap_wide_glyph_payload_stays_usable() {
    let home = TempHome::new();
    let wide = home.path().join("测 试");
    fs::create_dir_all(&wide).expect("wide dir");
    let mut session = spawn_mbx(home.path(), WinSize { rows: 24, cols: 12 });
    wait_for(&mut session, "> ");
    type_line(&mut session, "cd \"测 试\"");
    wait_for(&mut session, "> ");
    type_line(&mut session, "printf 'MBX_WRAP:wide-narrow-ok\\n'");
    wait_all(&mut session, &["\nMBX_WRAP:wide-narrow-ok", "> "]);
}
