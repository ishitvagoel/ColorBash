#[path = "common/mod.rs"]
mod common;

use common::*;
use mbx_pty::visible_text;
use std::fs;
use std::path::PathBuf;

fn echo_pair(session: &mut mbx_pty::PtySession, first: &str, second: &str) {
    type_line(session, &format!("echo {first}"));
    wait_all(session, &[&format!("\n{first}"), "> "]);
    type_line(session, &format!("echo {second}"));
    wait_all(session, &[&format!("\n{second}"), "> "]);
}

#[test]
fn enable_vs_default_off_leaves_histfile_unchanged() {
    let off = TempHome::new("inv-off");
    let on = TempHome::new("inv-on");
    let off_data = off.data_home();
    let on_data = on.data_home();
    let off_hist = off.histfile();
    let on_hist = on.histfile();
    let off_data_s = off_data.to_str().unwrap();
    let on_data_s = on_data.to_str().unwrap();
    let off_hist_s = off_hist.to_str().unwrap();
    let on_hist_s = on_hist.to_str().unwrap();

    let mut off_session =
        spawn_history_shell(off.path(), &disabled_env(off_data_s, off_hist_s, &[]));
    let mut on_session = spawn_history_shell(on.path(), &enabled_env(on_data_s, on_hist_s, &[]));
    wait_for(&mut off_session, "> ");
    wait_for(&mut on_session, "> ");
    echo_pair(&mut off_session, "alpha", "beta");
    echo_pair(&mut on_session, "alpha", "beta");
    wait_for_count(&mbx_bin(), &on_data, 2);

    let off_dump = dump_histfile(&mut off_session, off.path(), &off_hist);
    let on_dump = dump_histfile(&mut on_session, on.path(), &on_hist);
    assert_eq!(
        histfile_lines(&off_dump),
        histfile_lines(&on_dump),
        "enable must not add HISTFILE changes; off={off_dump:?} on={on_dump:?}"
    );
    assert!(
        !off_data.join("mbx/history.sqlite3").exists(),
        "disabled history must not create a store"
    );
    exit_and_wait(&mut off_session);
    exit_and_wait(&mut on_session);
}

#[test]
fn explicit_off_matches_unset_histfile() {
    let unset = TempHome::new("inv-unset");
    let zero = TempHome::new("inv-zero");
    let unset_data = unset.data_home();
    let zero_data = zero.data_home();
    let unset_hist = unset.histfile();
    let zero_hist = zero.histfile();
    let unset_data_s = unset_data.to_str().unwrap();
    let zero_data_s = zero_data.to_str().unwrap();
    let unset_hist_s = unset_hist.to_str().unwrap();
    let zero_hist_s = zero_hist.to_str().unwrap();

    let mut unset_session =
        spawn_history_shell(unset.path(), &disabled_env(unset_data_s, unset_hist_s, &[]));
    let mut zero_session = spawn_history_shell(
        zero.path(),
        &disabled_env(zero_data_s, zero_hist_s, &[("MBX_HISTORY", "0")]),
    );
    wait_for(&mut unset_session, "> ");
    wait_for(&mut zero_session, "> ");
    echo_pair(&mut unset_session, "alpha", "beta");
    echo_pair(&mut zero_session, "alpha", "beta");

    let unset_dump = dump_histfile(&mut unset_session, unset.path(), &unset_hist);
    let zero_dump = dump_histfile(&mut zero_session, zero.path(), &zero_hist);
    assert_eq!(histfile_lines(&unset_dump), histfile_lines(&zero_dump));
    assert!(!unset_data.join("mbx/history.sqlite3").exists());
    assert!(!zero_data.join("mbx/history.sqlite3").exists());
    exit_and_wait(&mut unset_session);
    exit_and_wait(&mut zero_session);
}

#[test]
fn external_clear_does_not_change_histfile() {
    let home = TempHome::new("inv-clear");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    echo_pair(&mut session, "alpha", "beta");
    wait_for_count(&mbx_bin(), &data_home, 2);
    let before = dump_histfile(&mut session, home.path(), &histfile);

    let output = run_history(&mbx_bin(), &["history", "clear"], &data_home);
    assert!(
        output.status.success(),
        "clear failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    wait_for_count(&mbx_bin(), &data_home, 0);
    let after = fs::read_to_string(&histfile).unwrap_or_default();
    assert_eq!(
        histfile_lines(&before),
        histfile_lines(&after),
        "clear must not rewrite HISTFILE; before={before:?} after={after:?}"
    );

    type_line(&mut session, "echo still-usable");
    wait_all(&mut session, &["\nstill-usable", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn external_delete_does_not_change_histfile() {
    let home = TempHome::new("inv-del");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    echo_pair(&mut session, "alpha", "beta");
    wait_for_count(&mbx_bin(), &data_home, 2);
    let before = dump_histfile(&mut session, home.path(), &histfile);
    let store = home.store_path();

    let output = run_history(&mbx_bin(), &["history", "delete"], &data_home);
    assert!(
        output.status.success(),
        "delete failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!store.exists(), "delete must unlink the sqlite file");
    let wal = PathBuf::from(format!("{}-wal", store.display()));
    let shm = PathBuf::from(format!("{}-shm", store.display()));
    assert!(!wal.exists(), "delete must unlink the WAL file");
    assert!(!shm.exists(), "delete must unlink the SHM file");
    let after = fs::read_to_string(&histfile).unwrap_or_default();
    assert_eq!(
        histfile_lines(&before),
        histfile_lines(&after),
        "delete must not rewrite HISTFILE; before={before:?} after={after:?}"
    );

    type_line(&mut session, "echo still-usable");
    wait_all(&mut session, &["\nstill-usable", "> "]);
    exit_and_wait(&mut session);
}

#[test]
fn seeded_histfile_is_not_rewritten_on_append() {
    let off = TempHome::new("inv-seed-off");
    let on = TempHome::new("inv-seed-on");
    fs::write(off.histfile(), "echo prior\n").expect("seed");
    fs::write(on.histfile(), "echo prior\n").expect("seed");
    let off_data = off.data_home();
    let on_data = on.data_home();
    let off_hist = off.histfile();
    let on_hist = on.histfile();
    let off_data_s = off_data.to_str().unwrap();
    let on_data_s = on_data.to_str().unwrap();
    let off_hist_s = off_hist.to_str().unwrap();
    let on_hist_s = on_hist.to_str().unwrap();

    let mut off_session =
        spawn_history_shell(off.path(), &disabled_env(off_data_s, off_hist_s, &[]));
    let mut on_session = spawn_history_shell(on.path(), &enabled_env(on_data_s, on_hist_s, &[]));
    wait_for(&mut off_session, "> ");
    wait_for(&mut on_session, "> ");
    assert_eq!(
        count_entries(&mbx_bin(), &on_data),
        0,
        "the first prompt must not record a seeded HISTFILE entry"
    );
    type_line(&mut on_session, "");
    wait_for(&mut on_session, "> ");
    assert_eq!(
        count_entries(&mbx_bin(), &on_data),
        0,
        "empty Enter after the first prompt must not record a seeded HISTFILE entry"
    );
    type_line(&mut off_session, "echo current");
    type_line(&mut on_session, "echo current");
    wait_all(&mut off_session, &["\ncurrent", "> "]);
    wait_all(&mut on_session, &["\ncurrent", "> "]);
    wait_for_count(&mbx_bin(), &on_data, 1);

    let off_dump = dump_histfile(&mut off_session, off.path(), &off_hist);
    let on_dump = dump_histfile(&mut on_session, on.path(), &on_hist);
    assert_eq!(histfile_lines(&off_dump), histfile_lines(&on_dump));
    assert_eq!(
        histfile_lines(&on_dump)
            .iter()
            .filter(|line| *line == "echo prior")
            .count(),
        1,
        "history -a must preserve prior entries; dump={on_dump:?}"
    );
    exit_and_wait(&mut off_session);
    exit_and_wait(&mut on_session);
}

#[test]
fn exit_flush_histfiles_match() {
    let off = TempHome::new("inv-exit-off");
    let on = TempHome::new("inv-exit-on");
    let off_data = off.data_home();
    let on_data = on.data_home();
    let off_hist = off.histfile();
    let on_hist = on.histfile();
    let off_data_s = off_data.to_str().unwrap();
    let on_data_s = on_data.to_str().unwrap();
    let off_hist_s = off_hist.to_str().unwrap();
    let on_hist_s = on_hist.to_str().unwrap();

    let mut off_session =
        spawn_history_shell(off.path(), &disabled_env(off_data_s, off_hist_s, &[]));
    let mut on_session = spawn_history_shell(on.path(), &enabled_env(on_data_s, on_hist_s, &[]));
    wait_for(&mut off_session, "> ");
    wait_for(&mut on_session, "> ");
    echo_pair(&mut off_session, "alpha", "beta");
    echo_pair(&mut on_session, "alpha", "beta");
    wait_for_count(&mbx_bin(), &on_data, 2);
    exit_and_wait(&mut off_session);
    exit_and_wait(&mut on_session);

    let off_dump = fs::read_to_string(&off_hist).unwrap_or_default();
    let on_dump = fs::read_to_string(&on_hist).unwrap_or_default();
    assert_eq!(
        histfile_lines(&off_dump),
        histfile_lines(&on_dump),
        "exit flush must stay identical; off={off_dump:?} on={on_dump:?}"
    );
}

#[test]
fn ignorespace_is_not_recorded() {
    let home = TempHome::new("adm-space");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("HISTCONTROL", "ignorespace")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, " echo hidden");
    type_line(&mut session, "echo visible");
    wait_all(&mut session, &["\nvisible", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(commands, vec!["echo visible".to_owned()]);
    let dump = dump_histfile(&mut session, home.path(), &histfile);
    assert!(
        !histfile_lines(&dump)
            .iter()
            .any(|line| line == " echo hidden"),
        "dump={dump:?}"
    );
    exit_and_wait(&mut session);
}

#[test]
fn leading_space_is_preserved_when_admitted() {
    let home = TempHome::new("adm-keepspace");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, " echo keptspace");
    wait_all(&mut session, &["\nkeptspace", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(commands, vec![" echo keptspace".to_owned()]);
    exit_and_wait(&mut session);
}

#[test]
fn ignoredups_records_one_consecutive_duplicate() {
    let home = TempHome::new("adm-dups");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("HISTCONTROL", "ignoredups")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo dup");
    wait_all(&mut session, &["\ndup", "> "]);
    type_line(&mut session, "echo dup");
    wait_all(&mut session, &["\ndup", "> "]);
    type_line(&mut session, "echo keep");
    wait_all(&mut session, &["\nkeep", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(
        commands,
        vec!["echo keep".to_owned(), "echo dup".to_owned()]
    );
    exit_and_wait(&mut session);
}

#[test]
fn histignore_is_not_recorded() {
    let home = TempHome::new("adm-ignore");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("HISTIGNORE", "rm *")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "rm temp.txt");
    type_line(&mut session, "echo keep");
    wait_all(&mut session, &["\nkeep", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(commands, vec!["echo keep".to_owned()]);
    exit_and_wait(&mut session);
}

#[test]
fn history_off_commands_are_not_recorded() {
    let home = TempHome::new("adm-off");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "set +o history");
    type_line(&mut session, "echo hidden");
    wait_all(&mut session, &["\nhidden", "> "]);
    type_line(&mut session, "set -o history");
    type_line(&mut session, "echo visible");
    wait_all(&mut session, &["\nvisible", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert!(
        commands.iter().any(|line| line == "echo visible"),
        "commands={commands:?}"
    );
    assert!(
        commands.iter().any(|line| line == "set +o history"),
        "commands={commands:?}"
    );
    assert!(
        !commands.iter().any(|line| line == "echo hidden"),
        "commands={commands:?}"
    );
    assert!(
        !commands.iter().any(|line| line == "set -o history"),
        "commands={commands:?}"
    );
    exit_and_wait(&mut session);
}

#[test]
fn history_dash_s_is_recorded_without_executing() {
    let home = TempHome::new("adm-dashs");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "history -s 'injected-marker'");
    type_line(&mut session, "echo real-output");
    let output = wait_all(&mut session, &["\nreal-output", "> "]);
    assert!(
        !visible_text(&output)
            .lines()
            .any(|line| line.trim() == "injected-marker"),
        "injected command must not execute; output={:?}",
        visible_text(&output)
    );
    wait_for_count(&mbx_bin(), &data_home, 2);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert!(
        commands.iter().any(|line| line.contains("injected-marker")),
        "commands={commands:?}"
    );
    assert!(
        commands.iter().any(|line| line == "echo real-output"),
        "commands={commands:?}"
    );
    exit_and_wait(&mut session);
}

#[test]
fn multiline_command_is_recorded_folded() {
    let home = TempHome::new("adm-ml");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell_rc(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[]),
        "PS2='CONT> '\n",
    );
    wait_for(&mut session, "> ");
    session
        .write_str("echo one \\\n", common::deadline(2))
        .expect("write");
    wait_for(&mut session, "CONT> ");
    session
        .write_str("two\n", common::deadline(2))
        .expect("write");
    wait_all(&mut session, &["\none two", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(commands, vec!["echo one two".to_owned()]);
    assert!(
        !commands.iter().any(|line| line.contains('\n')),
        "folded entry must not contain a newline; commands={commands:?}"
    );
    exit_and_wait(&mut session);
}

#[test]
fn empty_enter_does_not_duplicate_the_last_record() {
    let home = TempHome::new("adm-empty");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo first");
    wait_all(&mut session, &["\nfirst", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    type_line(&mut session, "");
    type_line(&mut session, "echo after-empty");
    wait_all(&mut session, &["\nafter-empty", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(
        commands,
        vec!["echo after-empty".to_owned(), "echo first".to_owned()]
    );
    exit_and_wait(&mut session);
}

#[test]
fn policy_exclusion_leaves_bash_history_intact() {
    let home = TempHome::new("adm-excl");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_EXCLUDE", "git *")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "git status");
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo keep");
    wait_all(&mut session, &["\nkeep", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    let commands = sidecar_commands(&mbx_bin(), &data_home);
    assert_eq!(commands, vec!["echo keep".to_owned()]);
    let dump = dump_histfile(&mut session, home.path(), &histfile);
    assert!(
        histfile_lines(&dump)
            .iter()
            .any(|line| line == "git status"),
        "Bash must still admit excluded commands; dump={dump:?}"
    );
    exit_and_wait(&mut session);
}
