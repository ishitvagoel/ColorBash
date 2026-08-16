# FND-001 slice: link green GitHub Actions CI on origin/main

Status: `complete` for C-1–C-2 (2026-08-16). Do not mark `G0` complete.
Remaining `G0` evidence is still the platform matrix, `HRD-001` macOS, and
representative `PRM-004` percentiles (`PRM-002` is `validation`). Remaining `G2`
is still the write-ack budget (foreign-user open and PRM-002 wrap recorded).

## Why this slice (do not pick a different leftover)

Remaining work, ranked. Implement **only row 1** in this change.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Link green CI for `FND-001` / `BST-005` (this plan) | `.github/workflows/ci.yml` already runs `bash tests/run.bash`. Immediate next work item 2. Docs-only. |
| 2 | Foreign-user open (`HIST-004` case 7 remainder) | Needs a **different host uid**. Do not fake `seteuid`. |
| 3 | Write-ack p95/p99 budget miss | Correctness recorded. Do not chase product-code latency unless a test proves the prompt waits on SQLite. |
| — | Editor / `G3` / fuzzy / default-on capture | Blocked on `G2` / `G3` / remaining `G0` matrix |

## Goal

1. Record a durable green GitHub Actions run URL for workflow `CI` on
   `origin/main` that executes `bash tests/run.bash`.
2. Mark `FND-001` and `BST-005` `complete` only when that run exists and its
   `headSha` is on `origin/main`.
3. Keep `G0` at `validation`. Do not claim the macOS/platform matrix.

## Out of scope (hard)

- Rewriting `.github/workflows/ci.yml`
- Weakening `tests/run.bash` to make CI green
- Marking `G0`, `G2`, or `HIST-007` complete
- Foreign-user open, write-ack product optimization, history storage changes
- Committing, pushing, or editing shell startup files unless asked

## Test cases (implement all)

| ID | Case | Assert |
| --- | --- | --- |
| C-1 | Green CI run on `origin/main` | Workflow `CI`, `conclusion=success`, runs `bash tests/run.bash`, URL recorded |
| C-2 | `FND-001` complete | Linked `headSha` is on `origin/main` |
| C-3 | `G0` remains `validation` | Docs still list platform matrix / `HRD-001` / `PRM-004` as open |

## Recorded evidence

- Repository: `ishitvagoel/ColorBash`
- Branch: `main`
- Commit: `8c8dad24d46d75d5eb311bacc06a0e2e25b5c5a9`
- Workflow: `CI` (`.github/workflows/ci.yml`)
- Event: `push`
- Conclusion: `success`
- Updated: 2026-08-16T08:50:58Z
- Run URL: https://github.com/ishitvagoel/ColorBash/actions/runs/31937499009

Reproduce probe:

```bash
curl -sS "https://api.github.com/repos/ishitvagoel/ColorBash/actions/runs?branch=main&per_page=5"
```

## Documentation updates (same change)

- `docs/roadmap.md` — Current baseline, `FND-001`, `BST-005`, Immediate next
  work item 2, changelog
- `docs/solid-hardening-checklist.md` — CI URL under Completion evidence
- This file

## Follow-on work (not this change)

1. `G0` platform matrix, `HRD-001` macOS PTY run, `PRM-004` representative
   percentiles (Darwin PTY constant cfg-split D-1–D-3 already recorded in
   `docs/hrd-001-darwin-pty-constants-plan.md`)
2. `HIST-007` remaining `G2`: write-ack percentile budget (foreign-user open
   recorded)
3. Write-ack budget only after a test proves SQLite is on the prompt path
