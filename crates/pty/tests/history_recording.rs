#[path = "common/mod.rs"]
mod common;

use common::*;

#[test]
fn admitted_commands_are_recorded_through_mbx2() {
    let home = TempHome::new("hrec");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo alpha");
    wait_all(&mut session, &["\nalpha", "> "]);
    type_line(&mut session, "echo beta");
    wait_all(&mut session, &["\nbeta", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    exit_and_wait(&mut session);

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
    let home = TempHome::new("hrec");
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
    exit_and_wait(&mut session);

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
    let home = TempHome::new("hrec");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &disabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo hidden");
    wait_all(&mut session, &["\nhidden", "> "]);
    exit_and_wait(&mut session);

    assert!(
        !data_home.join("mbx/history.sqlite3").exists(),
        "disabled history must not create a store"
    );
}

#[test]
fn hostile_command_text_remains_inert() {
    let home = TempHome::new("hrec");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(home.path(), &enabled_env(data_home_s, histfile_s, &[]));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo 'a%20b'");
    wait_all(&mut session, &["\na%20b", "> "]);
    type_line(&mut session, "printf 'x-tab-\\t-\\n'");
    wait_all(&mut session, &["\nx-tab-", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 2);
    exit_and_wait(&mut session);

    let recent = query(&mbx_bin(), &["history", "search", "recent"], &data_home);
    assert!(recent.contains("printf"), "recent={recent:?}");
}

#[test]
fn admitted_commands_record_repository_root_and_are_searchable_by_repo() {
    let home = TempHome::new("hrec");
    let git = which_git();
    assert!(
        std::process::Command::new(&git)
            .args(["init", "--quiet"])
            .current_dir(home.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new(&git)
            .args(["symbolic-ref", "HEAD", "refs/heads/hist-branch"])
            .current_dir(home.path())
            .status()
            .unwrap()
            .success()
    );
    let repo_root = git_show_toplevel(&git, home.path());
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &enabled_env(data_home_s, histfile_s, &[("MBX_DISABLE_GIT", "0")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo repo-ctx");
    wait_all(&mut session, &["\nrepo-ctx", "> "]);
    wait_for_count(&mbx_bin(), &data_home, 1);
    exit_and_wait(&mut session);

    let by_repo = query(
        &mbx_bin(),
        &["history", "search", "repo", &repo_root],
        &data_home,
    );
    assert!(
        by_repo.contains("echo repo-ctx"),
        "repo search should find the admitted command; root={repo_root:?} out={by_repo:?}"
    );
}

fn which_git() -> std::path::PathBuf {
    let mut name = std::ffi::OsString::from("git");
    name.push(std::env::consts::EXE_SUFFIX);
    let search =
        std::env::var_os("PATH").unwrap_or_else(|| std::ffi::OsString::from("/usr/bin:/bin"));
    for directory in std::env::split_paths(&search) {
        if directory.as_os_str().is_empty() || !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(&name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("git was not found on PATH");
}

fn git_show_toplevel(git: &std::path::Path, cwd: &std::path::Path) -> String {
    let output = std::process::Command::new(git)
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()
        .expect("git rev-parse");
    assert!(
        output.status.success(),
        "git rev-parse --show-toplevel failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .to_owned()
}
