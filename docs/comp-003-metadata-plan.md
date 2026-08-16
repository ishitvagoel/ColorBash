# COMP-003 slice: typed candidate metadata (K-1–K-4)

Status: `validation` (2026-08-16). `G4` is `complete`. `G3` is in `validation`.
This packet adds an additive kind/description side channel on the existing
stock-completion adapter. Insertion bytes stay stock. Do not mark `COMP-003`,
`COMP-001`, `COMP-002`, or `G3` complete.

Do not start ranking, popup, Git candidates, ghost, or highlighting.

## Why this slice

Immediate next work after `G4` gate close. ADR 0006: stock programmable
completion stays authoritative; provider kinds/descriptions are additive.
`COMP-004` popup stays blocked on `G3`. Fuzzy ranking is a later `COMP-003`
leftover, not this packet.

This is a **small product slice** (same size as N-1–N-2 / S-1–S-4), not a
gate close and not a G3 redraw experiment.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Typed candidate metadata (this plan) | `G4` complete; no popup required. |
| 2 | Bounded fuzzy ranking | Later `COMP-003` leftover. Needs the metadata model first. |
| — | `G3` continuous decoration | Unproven. Not a Composer feature slice (`docs/g3-decision-plan.md`). |
| — | Popup / `COMP-004` | Blocked on `G3`. |
| — | Git candidate kinds (`GIT-004`) | Blocked on this metadata model. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. After a wrapped `-F` adapter runs, parallel metadata arrays match
   `COMPREPLY` length. `COMPREPLY` insertion values are unchanged.
2. Fixture candidates get a kind string. Descriptions are optional and
   never inserted into the line.
3. Metadata is bounded and sanitized (C0/DEL stripped; length cap) before
   any later UI could consume it. This slice does not render a popup.
4. Default install still does not wrap stock `ls`/`printf` or define
   fixtures unless `MBX_COMP_FIXTURES=1`.
5. `COMP-003` may move to `validation`. Do not mark it `complete` (ranking
   remains). `G3` stays `validation`.

## Out of scope (hard)

- Fuzzy ranking, scoring, or reordering `COMPREPLY`
- Popup menu, keyboard navigation, or terminal rendering (`COMP-004`)
- Git candidate kinds or provider subprocesses (`GIT-004`)
- Ghost, highlighting, enhanced Ctrl+R
- Rebinding printable keys or taking Readline ownership (ADR 0003)
- Changing MBX1 framing or adding a protocol version
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Percentile benches
- Marking `COMP-003`, `COMP-001`, `COMP-002`, `G3`, or `EDT-001` complete
- Committing unless asked
- Creating a second plan file
- Widening into ranking or popup

## Method

Keep stock completion authoritative (ADR 0006). Add metadata **beside**
`COMPREPLY`, never inside it.

In `_mbx_comp_wrap_backend` (or a helper it calls after the backend
returns), fill:

- `_MBX_COMP_KINDS` — parallel to `COMPREPLY`; fixture flag candidates use
  kind `flag`; probe candidate uses kind `word`; missing kind is empty
- `_MBX_COMP_DESCS` — parallel optional descriptions; empty by default

Sanitize each description: strip C0 and DEL; cap at 64 characters; cap the
arrays at `COMPREPLY` length. Do not write metadata into `PS1`, the
terminal, or history.

Reuse `mbx_comp_probe` / `mbx_comp_flag` fixtures behind
`MBX_COMP_FIXTURES=1`. Observe insertion through `printf 'GOT:%s|\n' …`
(M-023). Module tests may read `_MBX_COMP_KINDS` / `_MBX_COMP_DESCS`
directly. PTY tests must still assert GOT insertion bytes (M-019:
`wait_all` for output plus next prompt). Do not wait on CPR/DSR.

Do not wrap `ls` or `printf`. Do not invent a second adapter.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| K-1 | Insertion bytes unchanged | `MBX_COMP_FIXTURES=1`. `mbx_comp_flag --mbx-co`, Tab, `X`, Enter. `\nGOT:--mbx-comp-flag X|` then `> `. Same as P-4. | validation — `metadata_preserves_flag_insertion_bytes` |
| K-2 | Kind recorded for wrapped `-F` | Module: drive `_mbx_comp_flag_adapter` with `COMP_WORDS=(mbx_comp_flag --mbx-co)`. `COMPREPLY[0]` is `--mbx-comp-flag`. `_MBX_COMP_KINDS[0]` is `flag`. `_MBX_COMP_KINDS` length equals `${#COMPREPLY[@]}`. | validation — `tests/bash/modules.bash` |
| K-3 | Description never inserted | Fixture sets a description containing `EXTRA`. PTY: `mbx_comp_flag --mbx-co`, Tab, Enter. GOT line is `--mbx-comp-flag` (with stock suffix); output must not contain `EXTRA`. | validation — `metadata_description_never_inserted` |
| K-4 | Bound and sanitize | Module: backend `COMPREPLY` plus a description containing `$`, backtick, a C0 byte, and 80 characters. After wrap, description has no C0/DEL/`$`/backtick/backslash; length `<= 64`. Arrays do not exceed `COMPREPLY` length. | validation — `tests/bash/modules.bash` |

If a measured GOT line differs, keep the host bytes in that row's Status
cell. Do not “fix” stock insertion to match the plan.

## Docs to update (this slice)

1. `docs/roadmap.md` — `COMP-003` `not-started` → `validation` after K-1–K-4
   evidence. Immediate next work: ranking leftover later; do not start
   popup / ghost. Changelog row. Do not mark `COMP-003` complete.
2. `docs/architecture.md` — completion adapter note: additive kinds/descs
   beside `COMPREPLY`; point at this plan.
3. This file — Status `ready` → `validation` after evidence lands. Status
   column: pending → validation plus test names.

## Remaining after this slice

`COMP-003` is `validation`, not `complete`. Bounded fuzzy ranking remains.
Popup stays blocked on `G3`. `GIT-004` stays blocked until this metadata
model exists. Continuous decoration stays unproven. The 5 ms leftover is
reviewed before `G5`. `HRD-001` still needs a macOS host.
