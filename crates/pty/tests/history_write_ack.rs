#[path = "common/mod.rs"]
mod common;

use common::*;
use std::os::unix::fs::PermissionsExt;

const SENTINEL: &str = "secret-ack-token";
const DEFAULT_BENCH_COMMANDS: usize = 200;

fn ack_bench_env<'a>(data_home: &'a str, histfile: &'a str) -> Vec<(&'a str, &'a str)> {
    enabled_env(data_home, histfile, &[("MBX_HISTORY_ACK_BENCH", "1")])
}

fn bench_command_count() -> usize {
    std::env::var("MBX_BENCH_COMMANDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| *count > 0)
        .unwrap_or(DEFAULT_BENCH_COMMANDS)
}

fn run_bench_commands(session: &mut mbx_pty::PtySession, count: usize) {
    for index in 0..count {
        let marker = format!("bench-{index}");
        type_line(session, &format!("echo {marker}"));
        wait_all(session, &[&format!("\n{marker}"), "> "]);
    }
}

fn percentile(sorted: &[u64], percentile: u64) -> u64 {
    let rank = (sorted.len() as u64 * percentile).div_ceil(100);
    let index = rank.saturating_sub(1) as usize;
    sorted[index.min(sorted.len() - 1)]
}

#[test]
fn bench_env_without_history_does_not_create_store_or_samples() {
    let home = TempHome::new("wack1");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let mut session = spawn_history_shell(
        home.path(),
        &disabled_env(data_home_s, histfile_s, &[("MBX_HISTORY_ACK_BENCH", "1")]),
    );
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo probe");
    wait_all(&mut session, &["\nprobe", "> "]);
    exit_and_wait(&mut session);

    assert!(
        !home.store_path().exists(),
        "bench env alone must not create a store"
    );
    assert!(
        !home.ack_samples_path().exists(),
        "bench env alone must not create ack samples"
    );
}

#[test]
fn admitted_commands_emit_digit_only_ack_samples_without_command_text() {
    let home = TempHome::new("wack2");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let samples_path = home.ack_samples_path();
    let mut session = spawn_history_shell(home.path(), &ack_bench_env(data_home_s, histfile_s));
    wait_for(&mut session, "> ");
    for index in 1..=7 {
        type_line(&mut session, &format!("echo cmd{index}"));
        wait_all(&mut session, &[&format!("\ncmd{index}"), "> "]);
    }
    type_line(&mut session, &format!("echo {SENTINEL}"));
    wait_all(&mut session, &[&format!("\n{SENTINEL}"), "> "]);
    exit_and_wait(&mut session);

    let samples = read_ack_samples(&samples_path);
    assert_eq!(samples.len(), 8, "expected one sample per admitted command");
    assert_ack_samples_digits_only(&samples_path, SENTINEL);
    let mode = std::fs::metadata(&samples_path)
        .expect("ack sample file metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "ack sample file must be user-only");
}

#[test]
fn ack_samples_finish_before_sqlite_drain() {
    let home = TempHome::new("wack3");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let samples_path = home.ack_samples_path();
    let mut session = spawn_history_shell(home.path(), &ack_bench_env(data_home_s, histfile_s));
    wait_for(&mut session, "> ");
    for index in 1..=3 {
        type_line(&mut session, &format!("echo ack{index}"));
        wait_all(&mut session, &[&format!("\nack{index}"), "> "]);
    }

    let samples = read_ack_samples(&samples_path);
    assert_eq!(
        samples.len(),
        3,
        "samples must be present at prompt return before SQLite drain"
    );
    wait_for_count(&mbx_bin(), &data_home, 3);
    exit_and_wait(&mut session);
}

#[test]
fn empty_enter_after_recorded_command_does_not_add_ack_sample() {
    let home = TempHome::new("wack4");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let samples_path = home.ack_samples_path();
    let mut session = spawn_history_shell(home.path(), &ack_bench_env(data_home_s, histfile_s));
    wait_for(&mut session, "> ");
    type_line(&mut session, "echo first");
    wait_all(&mut session, &["\nfirst", "> "]);
    assert_eq!(read_ack_samples(&samples_path).len(), 1);
    type_line(&mut session, "");
    wait_for(&mut session, "> ");
    assert_eq!(
        read_ack_samples(&samples_path).len(),
        1,
        "empty Enter must not add an ack sample"
    );
    exit_and_wait(&mut session);
}

#[test]
#[ignore = "HIST-004 prompt-boundary write-ack release benchmark; run via scripts/benchmark-history-write-ack.bash"]
fn measure_prompt_boundary_write_ack_percentiles() {
    let bin = mbx_bin();
    assert!(
        bin.is_file(),
        "mbx binary missing; build the workspace before running the write-ack benchmark"
    );
    let command_count = bench_command_count();
    let home = TempHome::new("wack5");
    let data_home = home.data_home();
    let histfile = home.histfile();
    let data_home_s = data_home.to_str().unwrap();
    let histfile_s = histfile.to_str().unwrap();
    let samples_path = home.ack_samples_path();
    let mut session = spawn_history_shell_production_timeouts(
        home.path(),
        &ack_bench_env(data_home_s, histfile_s),
    );
    wait_for(&mut session, "> ");
    run_bench_commands(&mut session, command_count);
    exit_and_wait(&mut session);

    let mut samples = read_ack_samples(&samples_path);
    assert!(
        samples.len() >= command_count,
        "expected at least {command_count} ack samples; got {}",
        samples.len()
    );
    samples.truncate(command_count);
    samples.sort_unstable();
    let p50 = percentile(&samples, 50);
    let p95 = percentile(&samples, 95);
    let p99 = percentile(&samples, 99);
    println!(
        "area=history_write_ack commands={command_count} p50_us={p50} p95_us={p95} p99_us={p99}"
    );
    assert_ack_samples_digits_only(&samples_path, SENTINEL);
}
