# ADR 0012: Defer macOS HRD-001 pairwise PTY from Strategy A MVP

Status: Accepted (2026-08-27)

## Context

`G5` / `HRD-001` requires real-PTY evidence across the supported Bash/OS matrix.
Linux L-1–L-5 (nested Bash, SSH prompt context, login shell, vim restore, and
`/usr/bin/tmux`) are recorded in `docs/hrd-001-linux-pairwise-plan.md` and
`crates/pty/tests/hrd001_linux.rs`. Darwin PTY constant cfg-splits are recorded
(D-1–D-3 in `docs/hrd-001-darwin-pty-constants-plan.md`).

The full macOS pairwise matrix still needs a macOS host. This development
environment is Linux-only. Faking Darwin PTY runs on Linux would not produce
valid release evidence.

The product owner authorized deferring macOS platform-matrix requirements for
the current milestone while completing every other roadmap deliverable that this
host can evidence.

## Decision

1. **Defer the macOS `HRD-001` pairwise leg** from Strategy A MVP / `G5` close.
   It is not a development blocker. Status is `deferred` with owner **G5
   revisit**, not `blocked`.
2. **Keep the Linux/WSL evidence** as the satisfied platform slice for this
   milestone. Do not remove or weaken L-1–L-5 tests.
3. **Do not fake macOS PTY results** on Linux. When a macOS host is available,
   run `cargo test -p mbx-pty --test hrd001_linux` equivalents on Darwin and
   record them in a follow-on plan without reopening Strategy A feature exits.
4. **Pair with existing deferrals.** Overlay/highlighting/dim paint (ADR 0003),
   `HRD-003` percentiles (`docs/latency-budget-deferral.md`), and `COMP-004`
   GUI overlay (`discovery`) remain out of this Strategy A MVP. `GIT-005` stays
   post-MVP.
5. **`G5` may close** for Strategy A MVP when every non-deferred `HRD-*` item
   is complete and the evidence inventory in `docs/g5-strategy-a-close-plan.md`
   is satisfied on this tree.

## Alternatives

- **Keep `HRD-001` `blocked` and `G5` open indefinitely:** rejected; Linux
  evidence is durable and macOS cannot progress without hardware.
- **Run macOS matrix in Linux CI with emulation:** rejected; invalid PTY
  evidence.
- **Drop macOS from the long-term product intent:** rejected; defer only, IDs
  kept for G5 revisit.

## Consequences

- `docs/roadmap.md` records `HRD-001` Linux `complete`, macOS `deferred`.
- `G5` and Phase 9 move to `complete` for Strategy A MVP on 2026-08-27.
- README and compatibility docs must not claim macOS release-matrix evidence.
- A future macOS host run is a **revisit** slice, not a reopen of ghost, search,
  or completion Strategy A exits.
