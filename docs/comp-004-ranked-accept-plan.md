# COMP-004 leftover: ranked-accept `bind -x` chord (A-1–A-5)

Status: `complete` (2026-08-16). Popup policy P-1–P-4 is recorded in
`docs/comp-004-popup-plan.md`. Tab stays stock. This packet adds an optional
explicit chord that inserts the top-ranked completion candidate at
`READLINE_POINT` without changing Tab, rebinding printables, or drawing an
overlay.

## Why this slice

Immediate next work after the popup policy decision. `COMP-004` is in
`discovery` with no GUI overlay. The only named `COMP-004` leftover that can
produce evidence on this host is the ranked-accept chord (`docs/comp-004-popup-plan.md`).
`G3` / `EDT-001` already proved non-destructive `bind -x` insert
(`bash/editor.bash`). `COMP-003` already fills `_MBX_COMP_ORDER` beside stock
`COMPREPLY`.

This is a **small product slice** (same size as `COMP-003` metadata/ranking),
not a decoration experiment and not `HIST-009`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Ranked-accept chord (this plan) | Named `COMP-004` leftover; policy recorded; `bind -x` pattern exists. |
| 2 | `GIT-004` Git completion kinds | Separate slice; needs stable ranked-reply seam first. |
| — | GUI overlay / keyboard menu | Unproven; would need continuous decoration or Readline ownership (ADR 0003). |
| — | Ghost / highlighting / enhanced Ctrl+R | Still blocked on after-every-key decoration (B-5). |
| — | `HIST-009` fuzzy history | UI-free Phase 3 exit; 100k bench; do not mix here. |
| — | `COMP-001` / `COMP-002` gate close | Separate validation slice after ranked accept. |
| — | Adapter 5 ms / `PRM-004` | `deferred`. |
| — | `HRD-001` macOS | Needs a macOS host. |

## Goal

1. After a wrapped completion run, record the top-ranked candidate text in
   `_MBX_COMP_RANKED_REPLY` (`COMPREPLY[_MBX_COMP_ORDER[0]]` when order is
   non-empty). `COMPREPLY`, Tab insertion bytes, and `_MBX_COMP_LAST_REPLY`
   (stock `COMPREPLY[0]`) stay unchanged.
2. Install an optional `bind -x` chord (default `\C-x\C-a`, overridable via
   `MBX_COMP_ACCEPT_KEYSEQ`) in emacs and vi-insert keymaps. Reuse the occupied-
   keyseq skip pattern from `bash/editor.bash` (`MBX_COMP_ACCEPT_OVERRIDE=1` to
   overwrite). Do not rebind Tab or printable editing keys.
3. The chord replaces the current whitespace-delimited word with
   `_MBX_COMP_RANKED_REPLY` when that word is a non-empty prefix of the ranked
   candidate. Never insert `_MBX_COMP_KINDS`, `_MBX_COMP_DESCS`, or scores.
   Never execute inserted text. Never splice after the prefix (`aa` + `aaflag`
   must not become `aaaaflag`).
4. When no ranked reply exists (empty `COMPREPLY`, no prior wrapped completion),
   the chord is a no-op on the line buffer.
5. Add fixture `mbx_comp_rank` behind `MBX_COMP_FIXTURES=1` only (M-037): backend
   returns `COMPREPLY=(zzflag aaflag)` for a word matching `aa*` so ranking
   prefers `aaflag` while stock `COMPREPLY[0]` stays `zzflag`.
6. `COMP-004` may stay `discovery` (overlay still unproven). Do not mark
   `COMP-004`, `COMP-001`, or `COMP-002` complete.

## Out of scope (hard)

- Reordering or mutating `COMPREPLY` for Tab insertion
- GUI overlay, candidate list rendering, or arrow-key navigation
- Rebinding Tab, arrows, or other printable keys (ADR 0003)
- Ghost, highlighting, enhanced Ctrl+R
- `HIST-009` / `GIT-004` product code
- Changing MBX1 framing
- Logging command text (`M-023`)
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Percentile benches
- Marking `COMP-004`, `COMP-001`, or `COMP-002` complete
- Committing unless asked
- Creating a second plan file
- Widening into overlay or continuous decoration

## Method

Read `bash/editor.bash`, `bash/completion.bash`, ADR 0003, ADR 0006, and
`docs/comp-003-ranking-plan.md`. Keep stock completion authoritative.

In `_mbx_comp_wrap_backend`, after `_mbx_comp_fill_ranking`:

- Set `_MBX_COMP_RANKED_REPLY` to `${COMPREPLY[_MBX_COMP_ORDER[0]]}` when
  `${#_MBX_COMP_ORDER[@]}` is non-zero; otherwise clear it.
- Leave `_MBX_COMP_LAST_REPLY=${COMPREPLY[0]:-}` unchanged.

Add `_mbx_comp_accept_ranked` (name flexible) that reads
`_MBX_COMP_RANKED_REPLY` and replaces the current whitespace-delimited word
in `READLINE_LINE` when that word is a non-empty prefix of the ranked
candidate. Advance `READLINE_POINT` to the end of the replacement. Clear
`_MBX_COMP_RANKED_REPLY` at the next prompt (`_mbx_render_prompt`).

Install from `_mbx_completion_install` (always, not fixture-gated). Default
keyseq `\C-x\C-a` must not collide with the editor default `\C-x\C-y`.

Fixture (fixtures only):

- `mbx_comp_rank` command printing `GOT:%s|` like other flag fixtures.
- Backend: when current word matches `aa*`, `COMPREPLY=(zzflag aaflag)`.
- Install only when `MBX_COMP_FIXTURES=1`.

Tests observe bytes through `printf 'GOT:%s|\n' …` (M-023). PTY waits use
`wait_all` for output plus next prompt (M-019). Do not wait on CPR/DSR.

## Test cases

| ID | Case | Assert | Status |
| --- | --- | --- | --- |
| A-1 | Ranked accept inserts top-ranked bytes | Module: run R-2 wrap backend; `_MBX_COMP_RANKED_REPLY` is `aaflag` while `COMPREPLY[0]` stays `zzflag`. PTY: `mbx_comp_rank aa` + Tab + accept chord + Enter → `\nGOT:aaflag|`. Tab without chord keeps `\nGOT:aa|`. | complete — `tests/bash/modules.bash`, `ranked_accept_inserts_top_ranked_bytes`, `ranked_accept_tab_without_chord_keeps_prefix` |
| A-2 | Tab insertion unchanged | PTY: `mbx_comp_flag --mbx-co` + Tab + Enter → `\nGOT:--mbx-comp-flag` (same as P-1 / R-1). `COMPREPLY[0]` order unchanged. | complete — `ranking_preserves_flag_insertion_bytes` |
| A-3 | No ranked snapshot → no-op | PTY: type `echo ok`, accept chord, Enter → `\nok` without ranked fixture text. | complete — `ranked_accept_without_snapshot_is_noop` |
| A-4 | Occupied keyseq skipped | PTY: pre-bind default keyseq; install without override → accept chord not bound; with `MBX_COMP_ACCEPT_OVERRIDE=1` → bound. | complete — `occupied_accept_chord_is_not_overwritten`, `occupied_accept_chord_override_installs` |
| A-5 | Metadata never inserted | PTY: ranked fixture path; `\nGOT:aaflag|` and output must not contain `EXTRA`. | complete — `ranked_accept_metadata_never_inserted` |
| A-6 | Stale unrelated word refused | Module + PTY: after ranking `aaflag`, current word `ok` is unchanged. Ctrl-U then `echo ok` + chord → `\nok`. | complete — `tests/bash/modules.bash`, `ranked_accept_refuses_stale_unrelated_word` |

If a measured result differs, record host bytes in the Status cell. Do not make
Tab follow `_MBX_COMP_ORDER`.

## Docs to update (this slice)

1. `docs/roadmap.md` — immediate next work; changelog row; `COMP-004` evidence
   note for A-1–A-5. Do not mark `COMP-004` complete.
2. `docs/architecture.md` — ranked-accept chord; point at this plan.
3. `README.md` — tryable ranked-accept chord (fixture + default keyseq).
4. This file — Status `ready` → `complete` after evidence; Status column updates.

## Validate

```bash
cargo test -p mbx-pty completion_harness -- --nocapture
bash tests/bash/modules.bash
bash tests/run.bash
```

## Remaining after this slice

`COMP-004` stays `discovery` until overlay strategy is resolved or scope is
explicitly narrowed in an ADR. GUI overlay remains unproven. `GIT-004` Git
completion kinds is the next completion-phase leftover. Ghost, highlighting,
and enhanced Ctrl+R stay blocked on continuous decoration. `HIST-009` fuzzy
history and `HRD-001` macOS matrix remain separate. `COMP-005` Strategy A
insert/fallthrough later closed without overlay
(`docs/comp-005-strategy-a-close-plan.md`).
