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
#[ignore = "M-065: confirmed open defect, not yet fixed. The overlay's \\e7/\\e8 \
            (DECSC/DECRC) save is an absolute screen position; drawing enough \
            rows to scroll the screen invalidates it, and the subsequent \\e[J \
            erases from the wrong origin, destroying the prompt and prior \
            output (reproduced below). A correct fix needs either DSR cursor-row \
            querying (which this codebase does not have) or a different \
            rendering strategy - both are a new ADR-level decision, not a \
            same-pass patch. Run explicitly with --ignored to reproduce."]
fn overlay_near_the_bottom_of_a_short_terminal_leaves_the_prompt_intact() {
    let home = TempHome::new("ov-scr1");
    // 6 rows total; typing a couple of blank lines first pushes the active
    // prompt down near the bottom, the case the overlay's fixed 24-row test
    // fixture (completion_harness.rs) never exercises.
    let mut session = spawn_overlay_shell(home.path(), 6, 60, EIGHT_CANDIDATE_PRELUDE);
    wait_all(&mut session, &["> "]);
    session
        .write_str("echo one\n", deadline(2))
        .expect("filler 1");
    wait_all(&mut session, &["\none", "> "]);
    session
        .write_str("echo two\n", deadline(2))
        .expect("filler 2");
    wait_all(&mut session, &["\ntwo", "> "]);
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
    let shown = wait_all(&mut session, &["cand0"]);

    session
        .write_all(OVERLAY_KEYSEQ, deadline(2))
        .expect("hide overlay");
    // Give the hide time to land before taking the final snapshot.
    let after_hide = wait_all(&mut session, &["> "]);

    let mut screen = Screen::new(6, 60);
    screen.apply(&shown);
    screen.apply(&after_hide);

    let lines = screen.lines();
    let prompt_lines: Vec<&String> = lines.iter().filter(|line| line.contains("> ")).collect();
    assert!(
        !prompt_lines.is_empty(),
        "the prompt line must still be visible somewhere on screen after the \
         overlay is shown and hidden near the bottom of a short terminal; \
         screen was:\n{}",
        lines.join("\n")
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("echo two") || line.contains("two")),
        "output printed before the overlay was ever shown must not be \
         destroyed by ED (\\e[J) run from the wrong saved cursor position; \
         screen was:\n{}",
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
