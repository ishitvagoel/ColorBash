# COMP-003 leftover: bounded completion ranking (R-1–R-4)

Status: `complete` (2026-08-16). `G4` is `complete`. `COMP-003` metadata K-1–K-4
is in `validation`. This packet adds additive scores and a display-order
permutation beside `COMPREPLY`. Insertion bytes and `COMPREPLY` order stay
stock. Do not start popup, Git candidates, ghost, or highlighting.

## Why this slice

Immediate next work after K-1–K-4. ADR 0006: stock programmable completion
stays authoritative; ranking is additive. `COMP-004` popup stays blocked on
`G3`. Do not pull in `HIST-009` fuzzy history ranking.

This is a **small product slice** (same size as K-1–K-4), not a G3 redraw
experiment.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Bounded ranking (this plan) | Named `COMP-003` leftover; metadata model exists. |
| 2 | `G3` continuous decoration | Unproven. Not a Composer feature slice. |
| — | Popup / `COMP-004` | Blocked on `G3`. |
| — | Git ranking (`GIT-004`) | Blocked until this score model exists. |
| — | `HIST-009` fuzzy history | Separate phase leftover; 100k bench. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. After a wrapped `-F` adapter runs, `_MBX_COMP_SCORES` is parallel to
   `COMPREPLY`. Scores are non-negative integers. `COMPREPLY` values and
   order are unchanged.
2. `_MBX_COMP_ORDER` is a permutation of `0 .. n-1` sorted by score
   descending, then original index ascending. Tab insertion still uses
   stock `COMPREPLY` order, not this permutation.
3. Ranking is deterministic prefix/substring scoring over at most 64
   candidates. Extra replies keep score `0` and stay in `COMPREPLY`.
4. Default install still does not wrap stock `ls`/`printf` or define
   fixtures unless `MBX_COMP_FIXTURES=1`.
5. `COMP-003` may move to `complete` after R-1–R-4 evidence. `G3` stays
   `validation`. Do not start `COMP-004`.

## Out of scope (hard)

- Reordering or mutating `COMPREPLY` insertion values
- Fuzzy / Levenshtein / history-backed scoring (`HIST-009`)
- Popup menu, keyboard navigation, or terminal rendering (`COMP-004`)
- Git candidate kinds or provider subprocesses (`GIT-004`)
- Ghost, highlighting, enhanced Ctrl+R
- Rebinding printable keys or taking Readline ownership (ADR 0003)
- Changing MBX1 framing or adding a protocol version
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Percentile benches
- Marking `G3`, `EDT-001`, `COMP-001`, or `COMP-002` complete
- Committing unless asked
- Creating a second plan file
- Widening into popup or G3 redraw

## Method

Keep stock completion authoritative (ADR 0006). Rank **beside**
`COMPREPLY`, never inside it.

After `_mbx_comp_fill_metadata` (or from it), fill:

- `_MBX_COMP_SCORES` — parallel integers. Score a candidate against
  `COMP_WORDS[COMP_CWORD]` (the current word): exact match `300`,
  prefix match `200`, substring match `100`, else `0`. Only the first 64
  replies are scored; later replies stay `0`.
- `_MBX_COMP_ORDER` — indices `0 .. n-1` sorted by score desc, then index
  asc. Length equals `${#COMPREPLY[@]}`.

Do not sort `COMPREPLY`, `_MBX_COMP_KINDS`, or `_MBX_COMP_DESCS`.

Reuse `mbx_comp_probe` / `mbx_comp_flag` fixtures behind
`MBX_COMP_FIXTURES=1`. Observe insertion through `printf 'GOT:%s|\n' …`
(M-023). Module tests may read scores and order directly. PTY tests must
still assert GOT insertion bytes (M-019: `wait_all` for output plus next
prompt). Do not wait on CPR/DSR.

Do not wrap `ls` or `printf`. Do not invent a second adapter.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| R-1 | Insertion bytes unchanged | `MBX_COMP_FIXTURES=1`. `mbx_comp_flag --mbx-co`, Tab, `X`, Enter. `\nGOT:--mbx-comp-flag X|` then `> `. Same as P-4 / K-1. | complete — `ranking_preserves_flag_insertion_bytes` |
| R-2 | Prefix scores higher | Module: backend `COMPREPLY=(zzflag aaflag)` with current word `aa`. `_MBX_COMP_SCORES` length equals 2. `aaflag` score `> ` `zzflag` score. `_MBX_COMP_ORDER[0]` is the index of `aaflag`. `COMPREPLY[0]` remains `zzflag`. | complete — `tests/bash/modules.bash` |
| R-3 | Bound 64 scored | Module: backend fills 80 distinct replies. `${#_MBX_COMP_SCORES[@]}` is 80. Indices `64..79` have score `0`. `${#_MBX_COMP_ORDER[@]}` is 80. `COMPREPLY` length stays 80. | complete — `tests/bash/modules.bash` |
| R-4 | Description still never inserted | PTY: `mbx_comp_flag --mbx-co`, Tab, Enter. GOT contains `--mbx-comp-flag`; output must not contain `EXTRA`. Same as K-3. | complete — `ranking_description_never_inserted` |

If a measured GOT line differs, keep the host bytes in that row's Status
cell. Do not “fix” stock insertion to match the plan.

## Docs to update (this slice)

1. `docs/roadmap.md` — `COMP-003` `validation` → `complete` after R-1–R-4
   evidence. Immediate next work: do not start popup / ghost; `G3` stays
   `validation`. Changelog row.
2. `docs/architecture.md` — ranking is additive scores/order beside
   `COMPREPLY`; point at this plan.
3. This file — Status `ready` → `complete` after evidence lands. Status
   column: pending → validation/complete plus test names.
4. `docs/comp-003-metadata-plan.md` — remaining ranking leftover points
   here. Do not rewrite K-1–K-4 IDs.

## Remaining after this slice

`COMP-003` is `complete` if R-1–R-4 pass. Popup stays blocked on `G3`.
`GIT-004` may use this score model later. Continuous decoration stays
unproven. The 5 ms leftover is reviewed before `G5`. `HRD-001` still
needs a macOS host.
