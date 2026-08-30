# Repository review and completion plan (2026-08-29)

Status: review record and **proposal**. This document does not change any
roadmap status, gate, or ADR. Items marked `PROPOSED` need the owner's decision
before an agent may act on them.

- Reviewed tree: `claude/repo-review-plan-7qpm0p` at `ee89541` (identical to
  `origin/main`)
- Reviewer environment: Linux 6.18, Bash 5.2.21, cargo/rustc 1.94.1, **uid 0**
- Evidence commands are recorded inline; every claim below was executed on this
  tree, not inferred from documentation.

## 1. What the product is

MBX is an interaction layer for interactive Bash, not a shell. Bash keeps the
parser, expansion engine, executor, and job controller; a Rust helper (`mbx`)
supplies presentation data over a Bash `coproc` using the MBX1 (prompt) and MBX2
(history record/query) framing. Every enhancement is insert-only: nothing
executes until the user presses Enter.

The delivered surface is:

| Capability | Enable | State on this tree |
| --- | --- | --- |
| Adaptive two-line prompt, Git/status/SSH/production/duration | source loader | working |
| Semantic colors, icon fallbacks, 16/256/truecolor | automatic | working |
| History sidecar (local SQLite, `.bash_history` untouched) | `MBX_HISTORY=1` | working |
| History-search insert / cycle / restore (`\C-xh`, `\C-xl`) | `MBX_HISTORY=1` | working |
| Ghost suffix + cycle | `MBX_GHOST=1` | working |
| Completion adapter, ranked accept/cycle | wrap a `-F` completer | working |
| Completion overlay | `MBX_COMP_OVERLAY=1` | works; safety unproven (§3.5) |
| Syntax highlighting | `MBX_HIGHLIGHT=1` | works; forks per keystroke (§3.4) |

The engineering discipline is unusually high for a prototype: 10 stated delivery
invariants, 13 ADRs, 59 logged mistakes (57 `Fixed`, 2 `Mitigated` and both are
accepted, documented debt), and a gate-based roadmap where `complete` requires
linked evidence rather than the existence of code.

## 2. Where it actually stands

Against the originating brief's MVP list (`CODEX_MODERN_BASH_ARCHITECTURE.md`
§45), all seven items exist in some form. Gates `G0`–`G5` are recorded
`complete` for Strategy A on Linux. The tree is genuinely green apart from one
environment-dependent test (§3.1):

```
cargo test --workspace -- --skip unreadable_store_fails_closed_without_widening
  → 146 + 0 + 5 + 6 + 44 + 6 + 14 + 7 + 21 + 4 + 14 + 15 + 5 + 25 + 4 + 5 + 9 passed, 0 failed
cargo clippy --workspace --all-targets -- -D warnings   → clean
bash -n bash/*.bash scripts/*.bash tests/**/*.bash      → clean
bash tests/bash/modules.bash target/debug/mbx           → PASS
bash tests/bash/smoke.bash target/debug/mbx             → PASS
bash tests/integration/protocol.bash "$PWD/target/debug/mbx" → PASS
```

The honest summary is that **the hard part is done**. What remains is not more
features; it is (a) two decoration features whose safety and cost are not yet
evidenced, (b) a product that no one outside this repository can install or
diagnose, and (c) several status/process deadlocks that will stall the next
agent regardless of how capable it is.

## 3. Findings

Ordered by how much they block completion, not by size.

### 3.1 `BLK-1` — the canonical suite cannot pass as root

`bash tests/run.bash` fails on this host at `cargo test --workspace`:

```
storage::tests::unreadable_store_fails_closed_without_widening
  panicked at crates/cli/src/storage.rs:2058:22: mode 0000 store must not open
```

The test chmods a store to `0000` and requires `QueuedHistoryStore::open` to
fail. Root holds `CAP_DAC_OVERRIDE`, so the open succeeds and the assert fires.
This is a test-portability defect, not a product defect: the sibling test
immediately above it (`storage.rs:2035`) already tolerates both `Ok` and `Err`
and asserts the invariant that actually matters — *the mode was never widened*.

Impact is larger than one red test. Root is the default uid in most
containerised CI images and in Claude Code's own remote environment, so **the
repository's single canonical verification command cannot be run there at all**,
and it fails early enough to skip Clippy and all three Bash suites. GitHub
Actions passes only because its runners are non-root.

### 3.2 `BLK-2` — documented test invocation fails with a relative path

`tests/integration/protocol.bash` runs one case inside a subshell that `cd`s
into a directory it then deletes, to prove non-prompt commands do not require a
working directory. It invokes `$MBX_TEST_BIN` from there, so a relative argument
no longer resolves:

```
$ bash tests/integration/protocol.bash target/debug/mbx
tests/integration/protocol.bash: line 22: target/debug/mbx: No such file or directory
```

`README.md` documents exactly that invocation. `tests/run.bash` passes an
absolute path, which is why this has never been caught.

### 3.3 `BLK-3` — CI does not cover the matrix the roadmap requires

`.github/workflows/ci.yml` is a single `ubuntu-latest` job on the runner's
default Rust. There is no MSRV job (the workspace declares `rust-version =
"1.85"` and README tells users to pin `1.85.0`), no Bash-version legs, no
release-profile build, and no macOS leg. `HRD-001` asks for a pairwise
Bash/OS/terminal matrix; none of it is automated, so the Linux legs that *are*
achievable today are re-proved by hand each time instead of by CI.

### 3.4 `BLK-4` — syntax highlighting forks a process on every keystroke

`bash/highlight.bash:232` runs, inside `_mbx_highlight_refresh`:

```bash
exec {output_fd}< <(exec "$MBX_BIN" highlight "$plain" --point "$point" 2>/dev/null)
```

`_mbx_highlight_refresh` is called from `_mbx_highlight_self_insert`,
`_mbx_highlight_backspace`, and the motion widgets — that is one `fork`/`exec`
of the helper **per printable keystroke**. Measured on this host with the
release binary, 200 sequential invocations of the exact highlight call took
0.439 s wall, i.e. **~2.2 ms per spawn** before any Bash process-substitution,
fd, or `bind -x` overhead is counted.

This contradicts the roadmap's own editing budget, which reads: *"Key-to-redraw
p99 <= 16 ms; **no external command on a cache-hit keystroke**"*. It is also
inconsistent with the sibling feature: ghost already routes through the warm
coprocess via MBX2 `QUERY` with generation/stale-skip discipline (ADR 0011,
`bash/ghost.bash:354`) and only falls back to a spawn when no coprocess is
attached. Highlighting has no coprocess path at all, because **MBX2 has no
highlight frame type** (`docs/protocol-mbx2.md` defines `RECORD`, `PING`,
`QUERY`, `CANCEL`, `RESULT` only).

This is the single highest-value structural item in the repository. It is the
real reason Phase 6 cannot honestly close, and it is being tracked as a deferred
percentile leftover when it is actually a missing transport path.

### 3.5 `BLK-5` — overlay cursor safety is unproven, and unprovable with today's harness

`_mbx_comp_overlay_refresh` (`bash/completion.bash:459`) saves the cursor with
`\e7` (DECSC), prints up to eight `\n`-prefixed rows to `/dev/tty`, then
restores with `\e8` (DECRC). `_mbx_comp_overlay_clear` emits a bare `\e[J` at
wherever the cursor happens to be.

The risk: DECSC stores an absolute screen position. When the prompt sits within
eight rows of the bottom, those newlines scroll the screen, the saved row now
holds different content, and `\e8` restores to the wrong place — after which
`\e[J` erases from that wrong origin, potentially destroying visible output
above the prompt. Terminal behaviour here is not uniform across emulators, so
this must be settled by evidence rather than argument.

It currently cannot be. The overlay PTY tests
(`crates/pty/tests/completion_harness.rs`) run at a fixed `rows: 24, cols: 80`
with the prompt near the top; there is no bottom-of-screen case and no
`SIGWINCH` case. More fundamentally, `crates/pty` has **no terminal screen
model** — `visible_text` strips escape sequences from a byte stream, so no test
can currently assert where the cursor ended up or what the screen contains.
ADR 0013 promises "terminal-safe rendering" for `COMP-004`; that claim has no
mechanism by which it could be evidenced.

### 3.6 `BLK-6` — `HLT-003` is deadlocked against its own plan

The roadmap names `HLT-003` the active workstream. `docs/hlt-003-hostile-gate-plan.md`
lists three ranked items: slice 1 (hostile corpus + strip round-trip) is
`complete`, slice 2 (PTY hostile execute-plain) is `complete`, and rank 3 is the
p99 highlight benchmark — which standing policy (`docs/latency-budget-deferral.md`)
defers and forbids blocking on. So the active workstream has **no non-deferred
next action**, yet Phase 6 stays `validation` and the roadmap instructs agents
not to mark it `complete`. An agent following the documents faithfully has
nowhere to go.

The repository has already set the precedent for resolving this twice: `G2`
closed with the write-ack percentile carried as a separate deferred leftover,
and `G4` closed with the 5 ms adapter overhead deferred the same way.

### 3.7 `BLK-7` — an open PR contradicts the documented status

PR #48 ("feat: opt-in interactive repo history search insert") is open, based on
`main@7c62926` which is now five commits behind, and reports
`mergeable_state: dirty` — it has a merge conflict. It implements `mbx repo
root` and `MBX_SEARCH_REPO=1`; neither exists on `main` (verified by grep).
Meanwhile `docs/roadmap.md` still records "interactive repo insert unauthorized"
in three places. Documentation and open work disagree, and the PR is rotting.

### 3.8 `RISK-1` — documentation load is now the throughput tax

`docs/` holds 83 markdown files plus 13 ADRs; the corpus is ~88,000 words
against ~23,000 lines of code and Bash. `docs/roadmap.md` alone is 79 KB and
`MISTAKES.md` is 64 KB, and `AGENTS.md` requires reading both *in full* before
planning or editing. Most of those 83 files are closed per-slice plans
(`comp-002-*`, `ghst-002-*`, `srch-003-*`, …) whose work is finished.

Every slice now costs a roadmap edit, a plan doc, an architecture edit, a README
edit, and possibly a MISTAKES entry. That discipline is what produced the
quality above — it should not be discarded — but the *read* cost is now paid by
every agent on every task, and it grows monotonically.

### 3.9 `GAP-1` — no one can install this

There are no git tags, no releases, and no prebuilt binaries. Installation is
`git clone` plus `cargo build --release` with Rust 1.85+. `HRD-004` explicitly
recorded "no package-manager installer" as an accepted state.

For a tool whose entire value proposition is *daily interactive comfort*, a
Rust toolchain requirement is the binding constraint on adoption. The product
is finished and unreachable at the same time.

### 3.10 `GAP-2` — the product reports state but does not diagnose

`mbx_status` (`bash/config.bash:48`) prints config path, helper path,
persist-bashrc, feature flags, `_MBX_*_BOUND` values, IPC mode, and the keymap
cheat sheet. That is genuinely useful and covers more than expected.

What it does not do is *diagnose*: no Bash version check, no tty/PTY-evidence
check, no color or Unicode capability report, no helper handshake round-trip, no
keybinding-collision report (which chords MBX skipped because they were already
bound, and how to override), no history store health (path, permissions, row
count, WAL state), and no actionable fix line for any failure. README currently
tells users to run `"$MBX_BIN" handshake`, `bind -X | grep _mbx_highlight_self_insert`,
and `printf 'bound=%s\n' "${_MBX_HIGHLIGHT_BOUND-}"` by hand.

The brief specifies `mbx doctor` (§41) and calls it "extremely useful", and §40
lists `mbx debug|config|theme`. **None of §40 or §41 has a roadmap ID** — this
scope is neither planned nor deferred; it is simply absent from the plan.

## 4. Completion plan

Five tracks. Track 0 is prerequisite; Tracks 1–2 close the two `validation`
features; Track 3 clears process debt; Track 4 is what turns a finished
prototype into a product someone can use. Each slice is sized to the
repository's existing "smallest coherent vertical slice" rule and names its exit
evidence.

Proposed new IDs are marked `PROPOSED`; they are not roadmap entries until the
owner accepts them.

### Track 0 — restore the canonical suite (do first; ~half a day)

| Slice | Work | Exit evidence |
| --- | --- | --- |
| `T0-1` | Make `unreadable_store_fails_closed_without_widening` root-safe: accept `Ok` or `Err` as the sibling test at `storage.rs:2035` already does, and assert the real invariant in both branches — mode still `0000`, file present, length unchanged, and on the `Ok` path that no widening occurred. | `bash tests/run.bash` green as uid 0 **and** as a normal user; new `MISTAKES.md` entry (privilege assumptions in permission tests) |
| `T0-2` | Resolve `MBX_TEST_BIN` to an absolute path at the top of `tests/integration/protocol.bash`. | `bash tests/integration/protocol.bash target/debug/mbx` passes from the repo root |
| `T0-3` | Extend CI: add an MSRV 1.85.0 job, a release-profile build, and Bash 5.0/5.1/5.2 legs for the three Bash suites. Add the macOS leg as an explicitly disabled/manual job so ADR 0012 has a place to land rather than a note. | green Actions run linked from `FND-001`/`BST-005`; roadmap `HRD-001` gains automated Linux legs |

Nothing else should start until `bash tests/run.bash` is green in a root
container, because until then no agent working in one can verify its own work.

### Track 1 — close Phase 6: move highlighting off the per-keystroke fork

This is the highest-value work in the plan.

| Slice | Work | Exit evidence |
| --- | --- | --- |
| `T1-1` `PROPOSED ADR 0014` | Decide the wire shape for keystroke-rate highlighting: either a new MBX2 `HIGHLIGHT`/`STYLED` frame pair, or a new `mode` on the existing `QUERY`/`RESULT` family. Reuse ADR 0011's generation counter and stale-RESULT skip verbatim — a keystroke path has exactly the same overlapping-reply hazard ghost has. MBX1 is untouched. | accepted ADR; `docs/protocol-mbx2.md` updated with the exact layout and bounds |
| `T1-2` | Helper side: add a narrow `HighlightHandler` port beside `HistoryHandler`, wired at the composition root, reusing `crates/cli/src/highlight.rs` unchanged. Same 64-KiB framing bounds, same fail-closed behaviour. | Rust transport tests mirroring the existing `mbx2_query_frames_*` cases; substitute-handler contract test |
| `T1-3` | Bash side: `_mbx_highlight_refresh` prefers `_mbx_engine_write` when a coprocess is attached and falls back to today's spawn otherwise — exactly the structure `bash/ghost.bash:354` already uses. Helper failure must still leave plain, usable bytes. | `tests/bash/modules.bash` contracts; `crates/pty/tests/highlight.rs` unchanged and still green |
| `T1-4` | Prove the budget structurally rather than statistically: a PTY test that types N printable characters with highlighting on and asserts **zero** additional `mbx` process executions (e.g. via a counting shim on `MBX_BIN`). Record a keystroke round-trip p50/p95/p99 alongside it. | new PTY case; benchmark record under `docs/benchmarks/` |
| `T1-5` | With `T1-4` recorded, put the `HLT-003` exit decision to the owner (§3.6): close `HLT-003` and Phase 6 with the p99 leftover carried as a separate deferred ID, per the `G2` write-ack and `G4` 5 ms precedents. | roadmap `HLT-003` and Phase 6 resolved with linked evidence |

`T1-4` is what makes this worth doing as engineering rather than bookkeeping:
"no external command on a cache-hit keystroke" is a *structural* property that a
test can assert deterministically, unlike a percentile that will always be
host-dependent and therefore always deferrable.

### Track 2 — close `COMP-004`: make the overlay provably terminal-safe

| Slice | Work | Exit evidence |
| --- | --- | --- |
| `T2-1` | Add a minimal VT screen model to `crates/pty`: a rows×cols grid plus cursor that applies CUP/CUU/CUD/CUF/CUB, ED/EL, DECSC/DECRC, and scroll-on-newline. This is a test-harness capability, not a product path, and it unblocks every future cursor-correctness claim — not just the overlay. | unit tests for the model itself; existing PTY suites unaffected |
| `T2-2` | Write the failing cases first: prompt on the last row at `rows: 8`, overlay shown then hidden, asserting the prompt is intact and no content above it was erased; and a `SIGWINCH` arriving while the overlay is visible. | reproducible red tests before any fix |
| `T2-3` | Fix per what `T2-2` shows. The likely shape is to stop relying on DECSC across a scroll: bound the row count by the space actually available below the cursor, degrade honestly (draw fewer rows, or refuse) when there is not enough, use relative motion with per-line `\e[K` instead of one blind `\e[J`, and clear on `SIGWINCH`. | `T2-2` green; `docs/comp-004-overlay-plan.md` updated |
| `T2-4` | Move `COMP-004` to `complete` with the type-to-filter GUI menu remaining explicitly `deferred`. | roadmap update with linked evidence |

If `T2-2` shows the current code is in fact safe on the tested emulators, that
is an equally good outcome — `COMP-004` closes on evidence instead of on
assertion, which is exactly what the repository's own status rules demand.

### Track 3 — clear process debt (cheap, unblocks everyone)

| Slice | Work |
| --- | --- |
| `T3-1` | Resolve PR #48: rebase onto current `main` and finish it, or close it and re-cut the slice. Either way, reconcile the three "interactive repo insert unauthorized" statements in the roadmap so open work and documented status agree. |
| `T3-2` | Archive closed plan docs into `docs/archive/` (keeping every file — the roadmap forbids silent deletion, and history stays readable). Reduce `docs/roadmap.md` to current status, gates, and next work by moving its completed-gate narrative and the bulk of the change log into `docs/archive/roadmap-history.md`. Target: an agent's mandatory reading drops from ~145 KB to something a session can hold alongside actual work. |
| `T3-3` | `PROPOSED`: ratify or retire the provisional performance budgets in one ADR. Five separate deferred percentile leftovers currently point at one deferral document; either the numbers are release promises or they are not, and the ambiguity is what produced `BLK-6`. |

### Track 4 — make it a product people can use

This is the track that most directly answers "complete the product", and none of
it is currently on the roadmap.

| Slice | Work | Exit evidence |
| --- | --- | --- |
| `T4-1` `PROPOSED DIAG-001` | `mbx doctor` (brief §41). Report and *diagnose*, each failure with a concrete fix line: Bash version vs 5.x support; interactive + real tty (the PTY-evidence rule users keep tripping on); `TERM`, color depth, `NO_COLOR`, Unicode/Nerd Font likelihood; helper path, `--version`, live handshake, and selected IPC mode; config file resolution and the effective flag set, including which env vars overrode the file; **keybinding collisions** — every chord MBX declined to take, why, and the matching `*_OVERRIDE` variable; history store path, directory/file permissions, row count, WAL state; and the ghost/highlight mutual-exclusion check. Re-point `mbx_status` at it as the short form. | Rust unit tests per probe; `tests/bash/modules.bash` contract; README replaces its manual `bind -X | grep …` recipes with one command |
| `T4-2` `PROPOSED REL-001` | Release path: tag `v0.1.0`; an Actions release job building Linux `x86_64` and `aarch64` binaries with checksums; `scripts/install.bash` prefers a verified downloaded binary and falls back to `cargo build` when none matches. Keep the existing rule that nothing writes `~/.bashrc` without `--bashrc`. | published release; install-from-release lifecycle case added to `docs/hrd-004-lifecycle-plan.md` |
| `T4-3` | Rewrite `README.md` around first use. The current README is an excellent reference manual and a poor introduction: ~24 KB, ten numbered feature walkthroughs, and roughly 60 environment variables before a new user sees why they would want this. Split it — a short README (what it is, install, comfort profile, `mbx doctor`, the four rules that always hold) plus `docs/reference.md` holding the current content, which is worth keeping intact. | — |

### Track 5 — remains deferred, with the reason stated

| Item | Why it stays deferred | What would change it |
| --- | --- | --- |
| macOS `HRD-001` pairwise matrix | needs a macOS host (ADR 0012) | a macOS runner, or the disabled CI job from `T0-3` being enabled |
| Dim after-every-key ghost paint | no accepted decoration hook | Track 1 makes the keystroke path cheap enough to reconsider; still needs its own ADR |
| Type-to-filter Ctrl+R overlay | same decoration constraint | Track 2's screen model would make it evidenceable for the first time |
| `GIT-005` provider SDK, v0.2/v0.3 scope | explicitly post-MVP | MVP shipped and adopted |

Two of the brief's seven MVP bullets — "live syntax highlighting" read as
*continuous dim paint* and "interactive completion menu" read as a *type-to-filter
popup* — remain unbuilt in their fullest form. That is a deliberate, documented
ADR 0003/0013 position, not an oversight, and Strategy A delivers a usable
version of both. It should stay a stated product decision rather than drift into
an implied promise.

## 5. Suggested order

1. **Track 0** — otherwise no agent in a container can verify anything.
2. **Track 1** — the largest genuine engineering gap, and it closes Phase 6.
3. **Track 4 `T4-1`** (`mbx doctor`) — can run in parallel with Track 1; it is
   independent and it is what makes the other features supportable.
4. **Track 2** — needs the harness work in `T2-1`, so it benefits from being
   after Track 1 rather than concurrent with it.
5. **Track 3** — fold `T3-1` in early (the PR rots further every week); `T3-2`
   is best done once, after Tracks 1–2 stop generating new plan docs.
6. **Track 4 `T4-2`/`T4-3`** — ship it.

## 6. What this review did not do

No feature code was written and no roadmap status was changed; `AGENTS.md`
requires a review-only task to report discrepancies rather than edit them. The
discrepancies reported for correction under a later authorized change are:
§3.6 (`HLT-003` has no non-deferred next action), §3.7 (roadmap says
"unauthorized" while PR #48 implements it), and §3.10 (brief §40/§41 tooling has
no roadmap ID in either the plan or the deferred list).

Two `MISTAKES.md` entries are warranted once fixes land, and are reported here
instead of added: **privilege assumptions in permission tests** (§3.1 — a test
asserting a DAC denial is invalid under `CAP_DAC_OVERRIDE`, and the correct
invariant is that permissions were never widened) and **relative binary paths
across a `cd` in test harnesses** (§3.2).
