# Phase 7 MVP close: Git/provider expansion

Status: `complete` (2026-08-25). `GIT-001`–`GIT-004` have durable evidence.
This packet closes Phase 7 for the Strategy A MVP: prompt status, bounded
Git acquisition, history root/branch, and completion kinds. `GIT-005`
generic SDK stays `deferred` post-MVP. Upstream/remotes/tags stay
unauthorized. Do not mark `SRCH-003`, `COMP-004`, or `G5` complete.

## Why this slice

`GIT-002` and `GIT-004` are the Phase 7 exit conditions and are already
`complete`. The phase row still said `validation` because
upstream/remotes/tags are unauthorized. That leftover is `GIT-005` /
post-MVP, not a missing MVP assert.

This is a **gate-close decision**. No product code.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Phase 7 MVP close (this plan) | Exits already evidenced. |
| — | Upstream/remotes/tags / generic SDK | `GIT-005` `deferred`; ADR 0007 update required. |
| — | `SRCH-003` complete | 100k leftover `deferred`; overlay `deferred`. |
| — | Overlay / highlighting | ADR 0003. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Evidence inventory (do not add tests)

| ID | Claim | Evidence | Status |
| --- | --- | --- | --- |
| GIT-001 | Typed prompt repository-status provider | `crates/cli/src/provider.rs`; ADR 0007; substitution/degradation tests | satisfied |
| GIT-002 | Deadline, cap, TTL cache | ADR 0007; provider tests; `docs/benchmarks/2026-08-15-solid-hardening.md` | satisfied |
| GIT-003 | Root/branch for history | `docs/hist-010-git-003-plan.md`; schema v3; `search repo` / `search branch` | satisfied for HIST-010 subset |
| GIT-004 | Completion kinds | `docs/git-004-kinds-plan.md`; `git_kinds_tab_keeps_prefix`; `git_kinds_ranked_accept_replaces_ref` | satisfied |
| GIT-005 | General provider SDK | Explicitly post-MVP | deferred — does not block MVP close |
| leftover | Upstream/remotes/tags | Unauthorized until a later consumer | deferred with `GIT-005` |

## Docs to update

1. `docs/roadmap.md` — Phase 7 `validation` → `complete` for MVP; `GIT-005`
   stays `deferred`. Changelog. Do not mark `G5` complete.
2. This file — Status `complete` after the roadmap edit.

## Remaining

`SRCH-003` stays `validation`. `COMP-004` stays `discovery`. `HRD-001` /
`G5` stay host-blocked. Do not start a generic SDK.
