//! Terminal-safety evidence for the completion overlay (`COMP-004`, ADR
//! 0013), using the `Screen` model added for T2 of
//! `docs/repo-review-2026-08-29.md`. `crates/pty/tests/completion_harness.rs`
//! proves the overlay draws and dismisses somewhere; these tests prove
//! *where* — specifically, that showing and hiding it near the bottom of a
//! small terminal does not scroll away or erase the prompt line above it,
//! which the overlay's blind `\e7`/`\e8` (DECSC/DECRC) plus `\e[J` could do
//! if the eight-row draw does not fit in the space actually left below the
//! cursor.

mod common;

use common::{TempHome, deadline, mbx_bin, path_env, wait_all, workspace_root};
use mbx_pty::{PtySession, Screen, SpawnOptions, WinSize};
use std::fs;
use std::path::Path;

const CTRL_X: u8 = 0x18;
const CTRL_U: u8 = 0x15;
const OVERLAY_KEYSEQ: &[u8] = &[CTRL_X, 0x0f];
const OVERLAY_DISMISS_KEYSEQ: &[u8] = &[CTRL_X, b'j'];
const TAB: u8 = 0x09;

/// Reads until the shell goes quiet, returning everything captured.
///
/// The overlay draws its selected row as `> cand0`, which contains the same
/// `"> "` the input anchor does, so waiting on a needle after a dismiss can
/// match the very rows the dismiss is supposed to erase and return before it
/// has happened. Draining on a timeout has no needle to be fooled by.
fn drain(session: &mut PtySession, seconds: u64) -> Vec<u8> {
    match session.read_until(deadline(seconds), mbx_pty::DEFAULT_CAPTURE_LIMIT, |_| false) {
        Ok(output) => output,
        Err(mbx_pty::PtyError::Timeout(captured)) => captured,
        Err(error) => panic!("draining the session failed: {error}"),
    }
}

fn spawn_overlay_shell(home: &Path, rows: u16, cols: u16, prelude: &str) -> PtySession {
    fs::write(
        home.join("rc.bash"),
        format!("{prelude}source \"${{MBX_TEST_ROOT}}/bash/init.bash\"\n"),
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
            .env("MBX_COMP_OVERLAY", "1")
            .env("COLUMNS", cols.to_string())
            .env("LINES", rows.to_string())
            .cwd(home)
            .winsize(WinSize { rows, cols }),
    )
    .expect("overlay screen shell spawn")
}

/// Eight candidates: the overlay's own cap, and enough to overflow a
/// terminal only a few rows tall below the prompt.
const EIGHT_CANDIDATE_PRELUDE: &str = "\
mbx_comp_many() { printf 'GOT:%s|\\n' \"$*\"; }
_mbx_comp_many_backend() {
    COMPREPLY=(cand0 cand1 cand2 cand3 cand4 cand5 cand6 cand7)
}
complete -F _mbx_comp_many_backend mbx_comp_many
";

#[test]
fn overlay_near_the_bottom_of_a_short_terminal_leaves_the_prompt_intact() {
    let home = TempHome::new("ov-scr1");
    // 6 rows total; typing a couple of blank lines first pushes the active
    // prompt down near the bottom, the case the overlay's fixed 24-row test
    // fixture (completion_harness.rs) never exercises.
    let mut session = spawn_overlay_shell(home.path(), 6, 60, EIGHT_CANDIDATE_PRELUDE);

    // Replay the *whole* session through the screen model. Applying only the
    // last couple of reads to a fresh `Screen` would model a terminal that
    // booted mid-session, so nothing printed earlier could ever be found and
    // the assertions below would be meaningless.
    let mut transcript: Vec<u8> = Vec::new();
    macro_rules! step {
        ($needles:expr) => {{
            let chunk = wait_all(&mut session, $needles);
            transcript.extend_from_slice(&chunk);
        }};
    }

    step!(&["> "]);
    session
        .write_str("echo one\n", deadline(2))
        .expect("filler 1");
    step!(&["\none", "> "]);
    session
        .write_str("echo two\n", deadline(2))
        .expect("filler 2");
    step!(&["\ntwo", "> "]);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_many\n", deadline(2))
        .expect("wrap");
    step!(&["> "]);
    session
        .write_str("mbx_comp_many cand", deadline(2))
        .expect("type prefix");
    step!(&["mbx_comp_many cand"]);
    session.write_all(&[TAB], deadline(2)).expect("tab");

    let mut before = Screen::new(6, 60);
    before.apply(&transcript);
    assert!(
        before
            .lines()
            .iter()
            .any(|line| line.contains("mbx_comp_many cand")),
        "fixture check: the typed prompt should be on screen before the \
         overlay is shown; screen was:\n{}",
        before.lines().join("\n")
    );

    session
        .write_all(OVERLAY_KEYSEQ, deadline(2))
        .expect("show overlay");
    step!(&["cand0"]);
    session
        .write_all(OVERLAY_KEYSEQ, deadline(2))
        .expect("hide overlay");
    transcript.extend_from_slice(&drain(&mut session, 2));

    let mut after = Screen::new(6, 60);
    after.apply(&transcript);
    let lines = after.lines();

    // The defect: showing eight rows under a prompt near the bottom scrolls
    // the screen, which invalidates the overlay's absolute `\e7` save. The
    // `\e8` on dismiss then restores a position that is no longer where the
    // overlay starts, so the following `\e[J` erases from the wrong origin —
    // leaving the drawn candidate rows stranded on screen and taking the
    // scrollback that was above them instead.
    //
    // Readline redraws the prompt line after the widget returns, so "is the
    // prompt visible" does *not* discriminate: it comes back either way. What
    // separates the two is whether the overlay's own rows were actually
    // erased. Against the unfixed code this screen reads
    // `cand2 cand3 cand4 cand5 cand6` above the prompt, with every earlier
    // line gone.
    let stranded: Vec<&String> = lines
        .iter()
        .filter(|line| (0..8).any(|n| line.contains(&format!("cand{n}"))))
        .collect();
    assert!(
        stranded.is_empty(),
        "dismissing the overlay must erase every row it drew; these were left \
         on screen: {stranded:?}\nfull screen was:\n{}",
        lines.join("\n")
    );

    // The prompt must also still be usable. This holds trivially when the
    // rows above were erased correctly, and is kept as a guard against a fix
    // that erases too much.
    assert!(
        lines.iter().any(|line| line.contains("mbx_comp_many cand")),
        "the prompt and its typed text must survive showing and hiding the \
         overlay near the bottom of a short terminal; screen was:\n{}",
        lines.join("\n")
    );

    session.write_str("exit\n", deadline(2)).expect("exit");
}

#[test]
fn resize_while_overlay_is_visible_leaves_a_usable_prompt() {
    let home = TempHome::new("ov-scr2");
    let mut session = spawn_overlay_shell(home.path(), 24, 80, EIGHT_CANDIDATE_PRELUDE);
    wait_all(&mut session, &["> "]);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_many\n", deadline(2))
        .expect("wrap");
    wait_all(&mut session, &["> "]);

    session
        .write_str("mbx_comp_many cand", deadline(2))
        .expect("type prefix");
    wait_all(&mut session, &["mbx_comp_many cand"]);
    session.write_all(&[TAB], deadline(2)).expect("tab");
    session
        .write_all(OVERLAY_KEYSEQ, deadline(2))
        .expect("show overlay");
    wait_all(&mut session, &["cand0"]);

    session
        .resize(WinSize { rows: 10, cols: 60 })
        .expect("resize while overlay is visible");
    session
        .write_all(OVERLAY_DISMISS_KEYSEQ, deadline(2))
        .expect("dismiss after resize");
    // Dismiss clears the overlay, not the typed line (by design); discard the
    // in-progress command before the sentinel so the sentinel is unambiguous.
    session
        .write_all(&[CTRL_U], deadline(2))
        .expect("clear line");

    // A sentinel command must still execute normally: the terminal state
    // (and the shell's own idea of the line) survived the resize.
    session
        .write_str("echo MBX_OVERLAY_RESIZE_OK\n", deadline(2))
        .expect("sentinel");
    wait_all(&mut session, &["\nMBX_OVERLAY_RESIZE_OK", "> "]);
    session.write_str("exit\n", deadline(2)).expect("exit");
}

const WIDE_CANDIDATE_PRELUDE: &str = "\
mbx_comp_wide() { printf 'GOT:%s|\\n' \"$*\"; }
_mbx_comp_wide_backend() {
    COMPREPLY=(WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ)
}
complete -F _mbx_comp_wide_backend mbx_comp_wide
";

#[test]
fn overlay_clamps_a_wide_row_so_it_does_not_wrap() {
    let home = TempHome::new("ov-scr-wide");
    let mut session = spawn_overlay_shell(home.path(), 10, 20, WIDE_CANDIDATE_PRELUDE);
    let mut transcript: Vec<u8> = Vec::new();
    macro_rules! step {
        ($needles:expr) => {{
            let chunk = wait_all(&mut session, $needles);
            transcript.extend_from_slice(&chunk);
        }};
    }

    step!(&["> "]);
    session
        .write_str("_mbx_comp_wrap_existing_f mbx_comp_wide\n", deadline(2))
        .expect("wrap");
    step!(&["> "]);
    // Two long candidates: Tab must not unique-insert a wrapping word into the
    // edit buffer. The overlay row is what must be clamped.
    session
        .write_str("mbx_comp_wide ", deadline(2))
        .expect("type prefix");
    step!(&["mbx_comp_wide "]);
    session.write_all(&[TAB], deadline(2)).expect("tab");
    session
        .write_all(OVERLAY_KEYSEQ, deadline(2))
        .expect("show overlay");
    transcript.extend_from_slice(&drain(&mut session, 2));

    let mut screen = Screen::new(10, 20);
    screen.apply(&transcript);
    let lines = screen.lines();
    assert!(
        lines.iter().any(|line| line.contains("mbx_comp_wide")),
        "the prompt must survive a wide overlay row; screen was:\n{}",
        lines.join("\n")
    );
    let wrap_continuations: Vec<&String> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && trimmed.chars().all(|ch| ch == 'W' || ch == 'Z')
                && !line.starts_with('>')
                && !line.starts_with("  ")
        })
        .collect();
    assert!(
        wrap_continuations.is_empty(),
        "a candidate wider than COLUMNS must be clamped to one row, not wrap \
         onto a continuation of W/Z; leftover rows: {wrap_continuations:?}\n\
         full screen was:\n{}",
        lines.join("\n")
    );

    session.write_str("exit\n", deadline(2)).expect("exit");
}
