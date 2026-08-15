# HIST-007 slice: prompt-boundary write acknowledgement

Status: `complete` for prompt-boundary write-ack correctness cases W-1–W-4 and
release percentile evidence W-5 (2026-08-16). The provisional write-ack budget
remains open on development WSL. Do not mark `G2` or `HIST-007` complete. WAL
crash/corrupt, foreign-user open, many-match prefix latency, and contention
cases 4, 5, and 8 remain later `G2` slices.

## Goal

Prove the `HIST-004` write budget at the **Bash prompt boundary**, not in-process
`record()`:

| Area | Budget | Measured at |
| --- | --- | --- |
| Write queue acknowledgement | p95 < 2 ms, p99 < 5 ms | Bash prompt boundary |

`ACK` means the helper accepted the record onto the bounded queue. It does not
mean SQLite committed. The in-process enqueue microbench in
`docs/benchmarks/2026-08-16-history-queries.md` is explicitly not this gate.

## Out of scope

- WAL `kill -9`, corrupt WAL/SHM, v0→v1 on 100k rows, foreign-user open
- Many-match `git` prefix index / schema v2
- Editor/G3, fuzzy ranking, repository context, default-on capture
- Changing MBX1/MBX2 framing, field counts, or ACK meaning
- Treating PTY `MBX_HISTORY_TIMEOUT=1.0` as the production budget
- Marking `G2` or `HIST-007` complete
- `set -euo pipefail` in sourced Bash modules
- Committing, pushing, or editing the user's shell startup files unless asked
- Reintroducing `MBX_DBG` or any diagnostic that copies command text (`M-023`)

## Method

Time the production prompt-path RECORD exchange with existing `_mbx_now_us`
(`bash/hooks.bash`; `EPOCHREALTIME` microseconds). Do **not** time PTY I/O,
Git, `PS1` render, `history 1` parse, or SQLite commit.

Window: immediately before `_mbx_protocol_encode_history_record` through a
successful `_mbx_protocol_decode_history_ack` inside `_mbx_history_record`.
Failed, timed-out, excluded, first-prompt, and drop-key-skip paths write no
sample.

Opt-in sample file, only when **both** `MBX_HISTORY=1` and
`MBX_HISTORY_ACK_BENCH=1`:

- Path: `$XDG_DATA_HOME/mbx/history-ack-samples` (same parent as the store).
- Format: one unsigned integer microseconds per line; nothing else.
- Mode: `0600` via `umask 077` in a create subshell (not external `chmod`);
  never print, log, or store command text, session id, history number, or cwd
  in this file.
- Append with Bash builtins only (`printf >>`). Do not spawn processes on the
  prompt path.

Reuse `crates/pty/tests/common/mod.rs` (`TempHome`, `spawn_history_shell`,
`wait_all`, `type_line`, `wait_for_count`, `exit_and_wait`). Read the sample
file from disk after output-plus-prompt (`M-019`). Never re-wait for `> `
after a match that may have consumed it.

The canonical suite keeps **correctness** PTY tests (small N). Percentiles are
a **release-mode** script, like `scripts/benchmark-history.bash`, not an
always-on unit test.

## Test cases

| ID | Case | Assert |
| --- | --- | --- |
| W-1 | `MBX_HISTORY_ACK_BENCH=1` with `MBX_HISTORY` unset | No store; no sample file (`M-024`) |
| W-2 | Both env vars set; first prompt then 8 short admitted commands | Exactly 8 sample lines of digits-only; sentinel command text (`secret-ack-token`) absent from the sample file, traces, and stderr |
| W-3 | After `wait_all` for command output plus `> `, read the sample file **before** `wait_for_count` | Sample count already matches admitted commands; proves the timer ends at ACK/prompt return, not SQLite drain |
| W-4 | Empty Enter after a recorded command | No extra sample line (`M-028`) |
| W-5 | Release-mode PTY, production timeouts (`MBX_HISTORY_TIMEOUT` / `MBX_IPC_TIMEOUT` default 0.10), `MBX_DISABLE_GIT=1`, ≥200 admitted `echo bench-{n}` commands after first-prompt skip, each synchronized with `wait_all` for the echoed marker and `> ` (`M-019`) | Record p50/p95/p99 µs and environment; compare to 2000 / 5000 µs. If over budget, record the miss; do not weaken the documented budget. Drive the helper through `MBX_TEST_BIN` pointing at the `release/mbx` cargo just built (`CARGO_TARGET_DIR` when set) |

## Edge cases

- First prompt must not emit a sample (`M-028`).
- Semantic PTY tests may keep `MBX_HISTORY_TIMEOUT=1.0`; the **percentile** run
  must use production 100 ms deadlines.
- Do not measure command-enter to next-prompt wall time. That includes render
  and PTY echo and is not the write-ack gate.
- Queue-full and helper-timeout remain drop-and-degrade; they are not samples.
- `bash/prompt.bash` remains the only `PS1` writer. The sampler must not touch
  `PS1`.
- Do not write `.bash_history` from MBX. Do not enable capture by default.
- If p95/p99 miss: leave `G2` open. Product-code changes only if a test proves
  the prompt waits on SQLite, samples contain command text, or ACK is delayed
  until commit. Do not switch to fire-and-forget ACK without an ADR.
- Stale docs still listing contention as remaining `G2` work
  (`docs/architecture.md`, `docs/protocol-mbx2.md`, ADR 0005 validation plan)
  should be reconciled in this change.

## Implementation checklist

1. Persist this plan (already done). Point `docs/roadmap.md` immediate next
   work at this slice. Do not mark `G2` complete.
2. Add the opt-in microseconds sampler in `bash/history.bash`. Keep sourced
   modules free of `set -euo pipefail`.
3. Add `crates/pty/tests/history_write_ack.rs` for W-1–W-4. Reuse common PTY
   helpers. Keep N small so `bash tests/run.bash` stays fast.
4. Add `scripts/benchmark-history-write-ack.bash` that builds `--release`,
   drives a genuine PTY (or a tiny `mbx-pty` bin test gated `--ignored`),
   reads the sample file, prints `area=history_write_ack ... p50_us= p95_us=
   p99_us=`, and fails the script (not the canonical suite) if p95 ≥ 2000 or
   p99 ≥ 5000.
5. Record environment and percentiles in
   `docs/benchmarks/2026-08-16-history-write-ack.md`.
6. Update `HIST-007` evidence notes only. If a confirmed defect is fixed, add
   or update `MISTAKES.md` in the same change.
7. Run focused PTY tests, then `bash tests/run.bash` with unsandboxed
   `/dev/ptmx` (`required_permissions: ["all"]`). Run the release write-ack
   script once and store the results file.

## Follow-on `G2` slices

WAL crash/corrupt + restart uniqueness; permission checks beyond mode bits;
many-match prefix latency; contention cases 4, 5, and 8.
