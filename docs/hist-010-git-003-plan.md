# HIST-010 / GIT-003 slice: repository context on history rows

Status: `complete` (2026-08-16). `HIST-009` fuzzy ranking exists. This packet
stores nullable `repo_root` / `repo_branch` on sidecar rows. Overlay, ghost,
highlighting, and Ctrl+R stay blocked. Do not log command text (M-023).

## Goal

1. Schema v3 adds nullable `repo_root` and `repo_branch`. Forward-only migrate
   from v1/v2. Existing rows stay NULL.
2. MBX2 RECORD is unchanged. The writer enriches from absolute `start_cwd`
   using the ADR 0007 Git adapter (`rev-parse --show-toplevel`, then
   `symbolic-ref --short HEAD` with `rev-parse --abbrev-ref HEAD` fallback).
3. Git lookup happens **before** `BEGIN IMMEDIATE`. Timeout, disable
   (`MBX_DISABLE_GIT=1`), relative cwd, or failure leave NULLs and still insert
   the history row. Writer cache: 128 entries, 1 s TTL, including absence.
4. Default `QueuedHistoryStore::open` uses a null context provider so 100k
   ingest and CLI search stay process-free. The helper serve path injects Git.
5. `mbx history search repo ROOT` is a bounded exact-root query. CLI stdout
   remains command text only.
6. Upstream/remotes/tags stay unauthorized until a later consumer.

## Remaining

Overlay / ghost / highlighting / Ctrl+R stay blocked. `HRD-001` macOS remains
Phase 9. Do not mark `COMP-004` complete. CLI `search branch` / `search failed`
leftovers are in `docs/hist-010-cli-filters-plan.md`.
