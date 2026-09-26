# Next-steps TODO — post-TUI (2026-09-26)

Working TODO built from everything that remains after `TUI-001..004`,
`ONBD-001..003`, and the chord-surface regression tests (`415c31a`).
Canonical status stays in `docs/roadmap.md`; this document drove execution
and records evidence as items complete.

**Final status (2026-09-26):** every session-actionable item is complete
(A1–A4, B1–B3, C via `TUI-005..007`, D1, D7) with evidence below and the
canonical suite green. On the owner's confirmation, D2–D6 remain out of
scope for this effort: D2 waits for the Contabo box to be scheduled (the
commands are documented here and in the 2026-09-03 session), D3 needs a
macOS host, D4 needs a Readline redisplay technique the prior evidence says
does not exist, D5 stays post-MVP policy, and D6 is monitoring-only. They
stay tracked in `docs/roadmap.md` (deferred/G5-revisit) and reopen the
moment their gate clears. Status legend: `[ ]` open,
`[x]` done (with evidence), `[~]` in progress, `[>]` blocked/decision-gated.

---

## A. Release readiness — the REL-001 closure path

The biggest remaining product milestone. The pipeline exists and was
dry-run-packaged on 2026-08-31; what is missing is the maintainer smoke run
of `release.yml` and the actual tag. Cutting the tag itself is the product
owner's decision — everything up to it can be prepared and verified.

- [x] **A1 — Local package dry-run from the current tree.** Re-run
  `scripts/package-release.bash`; assert the tarball contains `mbx`,
  `README.md`, `LICENSE-MIT`, `LICENSE-APACHE` and that `sha256sum -c`
  passes. Evidence: command output captured below.
- [x] **A2 — Maintainer smoke of `release.yml` via `workflow_dispatch`.**
  `gh workflow run` (build-only: the publish job is `refs/tags/v*`-gated,
  M-071, so this cannot publish). Watch the run to green. Evidence: run URL
  and conclusion.
- [x] **A3 — First-tag runbook.** A short, exact checklist (tag name
  `v0.1.0` to match `Cargo.toml`, the two commands to run, what to verify on
  the GitHub release page, how to smoke the tarball on a clean machine) so
  the owner can cut the release with one decision. Lives in this document
  (section at the end) and linked from the roadmap `REL-001` row.
- [x] **A4 — Roadmap reconciliation.** Update the `REL-001` row and change
  log with A1/A2 evidence in the same change as the work.

## B. TUI integration polish — gaps found while planning

- [x] **B1 — `mbx_configure` interactive menu cannot toggle the TUI.**
  `tui` is wired through defaults/`KNOWN`/env-mapping/`normalize`/
  `write_config`, but `print_menu` shows no entry, so an interactive user
  cannot enable or disable the picker. Add a menu row **without renumbering
  existing options** (docs and the help text reference "option 15" for
  persist; smoke tests may reference numbers — verify first). Plan: add as
  option `16)` in the Features block (numbers need not be contiguous).
  Handle `16` in the choice loop, keep `normalize()` semantics (tui on
  forces history on; history off forces tui off). Evidence: manual menu
  run + smoke/module test updates.
- [x] **B2 — `mbx_doctor` is unaware of the TUI.** Two diagnosable states
  are missing: (a) `MBX_TUI=1` but `MBX_HISTORY!=1` — picker can never
  launch (warn: enable history); (b) `MBX_TUI=1` with a helper binary that
  predates the `tui` subcommand — the chord will fall back silently to the
  inline widget (warn: rebuild the helper). Add both checks with fix lines,
  following the existing `_mbx_doctor_line` patterns. Evidence: module
  contracts D-style cases in `tests/bash/modules.bash`.
- [x] **B3 — Comfort-profile regression coverage for `MBX_TUI=1`.** The
  install comfort profile now sets `MBX_TUI=1`; confirm the install smoke
  cases assert the full comfort variable set (add `MBX_TUI` if the test
  enumerates variables). Evidence: updated smoke assert.

## C. TUI completion picker — COMPLETE (2026-09-26, per C1)

The owner directed completion of all todos, adopting the C1 recommendation.

- **C1 (recommended): complement, don't replace.** New opt-in
  `MBX_COMP_TUI=1`; after Tab on a wrapped completer, a new chord
  (`Ctrl-X t`, following the existing `Ctrl-X` letter convention and the
  occupied-skip rule) opens a modal picker over the ranked snapshot
  (`_MBX_COMP_RANKED_LIST`), exactly the history picker's machinery. The
  below-prompt overlay stays as the lightweight path; both share one
  snapshot. Pros: no behavior change for existing users, reuses everything.
  Cons: one more chord.
- **C2: replace the overlay.** The overlay toggle chord launches the picker
  instead. Pros: one interaction to learn. Cons: removes a shipped feature,
  and the overlay's at-a-glance listing is cheaper than a modal for 2-3
  candidates.

- [x] **`TUI-005` — `mbx tui complete` Rust core.** Candidates on stdin
  (bounded 512 rows / 4096 bytes per row, empties and over-long rows dropped
  whole), same modal loop, case-insensitive substring filter, title
  `MBX completions`. Evidence: 10 unit tests in `crates/cli/src/tui.rs`.
- [x] **`TUI-006` — Bash chord wiring.** `_mbx_comp_tui` +
  `_mbx_comp_install_tui` in `bash/completion.bash`: opt-in `MBX_COMP_TUI=1`,
  default `\C-xt` with occupied-skip and `MBX_COMP_TUI_OVERRIDE`; candidates
  piped from `_MBX_COMP_RANKED_LIST`; exit 2 = cancel leaves the line; C0/DEL
  gate; the pick replaces the word at the cursor with the snapshot refreshed
  at accept time. `configure.bash` menu option 17; `mbx_help`/`mbx_status`
  rows.
- [x] **`TUI-007` — evidence.** Module contracts (unset installs nothing,
  bound flag, handler no-op without a snapshot) and PTY
  `tui_chord_opens_picker_and_accept_replaces_the_word` in
  `crates/pty/tests/completion_harness.rs`: `Ctrl-X t` after wrapped Tab
  opens the picker on the alternate screen, type-to-filter to `1 match`,
  Enter replaces the word (`mbx_comp_rank zzflag`), execution only on the
  user's own Enter (`GOT:zzflag|`). One real defect found and fixed during
  this slice: the `set -u`-unsafe `_MBX_COMP_SNAPPED` reference, and a
  product decision recorded here — picker accept replaces the current word
  regardless of prefix (explicit user choice), unlike ranked-accept whose
  M-039 guard stays.

## D. Blocked / decision / hardware-gated — confirmed out of scope (owner, 2026-09-26)

The items below are tracked in `docs/roadmap.md` and reopen when their gate
clears; they are intentionally not part of this effort's completable scope.

- [x] **D1 — First `v*` tag.** Owner decision given 2026-09-26; executed per
  the A3 runbook. Tag `v0.1.0` at `a24b38a`; tag-triggered run 36261206838
  `success`; release published with 4 assets; downloaded tarball
  checksum-verified and `mbx --version` = 0.1.0. `REL-001` moved to
  `complete` and `scripts/install.bash --download` shipped (verified
  end-to-end: download → checksum → install → `mbx --version`; `--no-build`
  still skips everything; failed download + `--no-build` fails loudly). One
  regression introduced and caught by the canonical suite during this work:
  the first refactor dropped the `NO_BUILD` guard around the non-interactive
  build call, breaking `install --no-build`; fixed and smoke re-run green.
- [>] **D2 — Deferred percentiles** (`HRD-003`/`PRM-004`/write-ack):
  measurement work for the Contabo box; owner has not yet scheduled it
  (earlier instruction: stay on the PC). The commands are documented in the
  2026-09-03 session and `scripts/benchmark-*.bash` is ready — say the word
  and it runs.
- [>] **D3 — macOS `HRD-001` matrix and Darwin TUI port.** Needs a Mac
  (ADR 0012); the TUI terminal layer is Linux-gated with a clean non-Linux
  error by design.
- [>] **D4 — Dim ghost paint.** Still gated on Readline redisplay
  ownership; unchanged by ADR 0016, which deliberately stayed modal.
- [>] **D5 — `GIT-005` provider SDK.** Post-MVP policy.
- [>] **D6 — M-075 PTY flake.** Mitigated, root cause unreproduced;
  monitor. If it recurs, the rebuilt failure report names the cause.
- [x] **D7 — Housekeeping: delete the merged `fix/chord-history-leak`
  remote branch.** Deleted 2026-09-26 (`git push origin --delete`).

## Execution order

A1 → B1 → B2 → B3 → A2 (remote smoke, once local state is final) → A3/A4 →
full canonical suite → report. C and D are reported, not executed.

---

## Evidence log (filled as items complete)

### A1 — package dry-run (2026-09-26)

`cargo build --release --workspace` then
`MBX_PACKAGE_OUT=/tmp/rel-dryrun bash scripts/package-release.bash 0.1.0 x86_64-unknown-linux-gnu`
from `415c31a`+working tree: tarball 1,371,950 bytes; `sha256sum -c` OK;
contents exactly `mbx/`, `README.md`, `LICENSE-MIT`, `LICENSE-APACHE` under
`mbx-0.1.0-x86_64-unknown-linux-gnu/`. Release binary sanity: `mbx 0.1.0`;
`mbx tui search` without a tty fails cleanly with
`mbx tui needs a terminal on stdin`.

### B1 — configure menu TUI toggle (2026-09-26)

`print_menu` gained `16) History search TUI (needs history; Ctrl-X h opens
the picker)` in the Features block — existing options 1–15 keep their
numbers (README's "option 15" reference stays true). `handle_choice` case
`16` toggles with a one-line hint; `normalize()` already enforces
tui→history. Driven end-to-end: `16` + `w` writes
`[[ ${MBX_TUI+x} ]] || export MBX_TUI=1` and forces history on. Smoke menu
drives (`1\nw`, `4\nw`) unaffected.

### B2 — mbx_doctor TUI diagnostics (2026-09-26)

New rows in `mbx_doctor` (bash/config.bash): `MBX_TUI=1` without history →
`[WARN]` with the enable fix; helper predating the `tui` subcommand
(probed via `"$MBX_BIN" tui search </dev/null`, pipefail-safe capture) →
`[WARN]` with the rebuild fix; otherwise `[OK] ... full-screen picker`.
Missing-helper falls to OK here because the handshake section already
FAILs it. `mbx --help` usage now lists `mbx tui search [--seed TEXT]`.
Module contracts D-6 cover all four states (warn-no-history, warn-stale,
ok, absent-when-unset).

### B3 — comfort-profile coverage (2026-09-26)

`tests/bash/smoke.bash` now asserts the comfort profile writes
`MBX_TUI=1` alongside `MBX_HISTORY=1`.

### A2 — release workflow maintainer smoke (2026-09-26)

`gh workflow run Release --ref main` → run `36259115227`
(https://github.com/ishitvagoel/ColorBash/actions/runs/36259115227),
`workflow_dispatch` build-only (publish job is `refs/tags/v*`-gated, M-071).
**Result: `success`** — both `Build (x86_64-unknown-linux-gnu)` and
`Build (aarch64-unknown-linux-gnu)` green; the publish job was correctly
absent (tag-gated). This is the maintainer smoke `REL-001` was waiting on.

### Validation (2026-09-26)

Full canonical suite (`bash tests/run.bash`) green, including the new
contracts. One transient observation, not a defect: during the first
full-suite pass (while the release workflow watch was also running), the
M-072 differential deadline case reported a physically impossible negative
delta — a measurement stall under concurrent load. It did not reproduce on
the immediate re-run or in the focused suite; M-072's entry already records
this exact wall-clock exposure as systemic. Nothing recorded.

---

## First-tag runbook (A3) — cutting v0.1.0

Everything up to the tag is verified; the release is one decision.

1. **Confirm the smoke is green** (A2's run above, and CI on `main`).
2. **Tag and push** (maintainer action):

   ```bash
   git tag -a v0.1.0 -m "mbx 0.1.0 — first release" main
   git push origin v0.1.0
   ```

   `release.yml` then builds `x86_64` and `aarch64` Linux on native runners,
   packages both checksummed tarballs, and publishes the GitHub release —
   including the publish job, which only runs for `refs/tags/v*`.
3. **Verify the release page**: two tarballs + two `.sha256`; release notes
   from the tag message; no `workflow_dispatch`-sourced artifacts.
4. **Smoke a tarball on a clean machine** (Contabo is ideal):

   ```bash
   curl -LO https://github.com/ishitvagoel/ColorBash/releases/download/v0.1.0/mbx-0.1.0-x86_64-unknown-linux-gnu.tar.gz
   tar xzf mbx-0.1.0-*.tar.gz && cd mbx-0.1.0-*
   ./mbx --version && ./mbx doctor || true   # helper path check
   ```

5. **Afterwards** (separate change): un-defer the download-preferring half
   of `scripts/install.bash` and move `REL-001` to `complete` with the run
   and tarball URLs as evidence.

Why `v0.1.0`: matches `Cargo.toml` (`crates/cli` 0.1.0) and the project's
prototype stage; semver allows 0.x breaking moves later.
