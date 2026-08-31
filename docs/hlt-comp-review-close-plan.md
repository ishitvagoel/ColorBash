# ADR 0013 review close (highlight wrap + overlay leftovers)

Status: `complete` (2026-08-27). Slices 1–3 implemented on
`cursor/hlt-comp-overlay-741c`. H-1–H-6, O-1–O-5, and M-1 have module/PTY
evidence. Later work closed the leftover gates: ADR 0015 (2026-08-31) for
`HLT-002`/`HLT-003`/Phase 6; overlay width guard for `COMP-004`. Historical
"do not mark complete" notes below stay as the close-plan's original stop
condition, not current roadmap status.

Existing files (`bash/highlight.bash`, `bash/completion.bash`, `mbx highlight`)
are a first cut. They are not exit evidence. A PTY session with `MBX_HIGHLIGHT=1`
sets `_MBX_HIGHLIGHT_BOUND=1` while `bind -X` contains **no** `_mbx_highlight_*`
widgets. `highlighted_line_executes_plain_bytes` still passes because stock
typing and stock Enter already run `printf`. Overlay toggle after Tab does draw
in a PTY; dismiss, cap, sanitize, and ADR draw semantics do not.

## Why this plan

The review listed defects that make highlighting a no-op and leave overlay
short of ADR 0013. Those leftovers **defer** `HLT-003` latency/hostile gates
and dim paint. They must not be silently absorbed into `validation` without
named asserts.

| Rank | Item | Why this order |
| --- | --- | --- |
| 1 | **HLT wrap + safety + one-Enter PTY** (this plan’s implement-now slice) | Without widgets, C0, Enter, and motion tests cannot distinguish a no-op from the feature (M-038 / M-040). Wrap, strip-then-compare, job isolation, and Enter must land together. |
| 2 | Overlay dismiss / 8-row cap / candidate sanitize / draw | Overlay already toggles in PTY. Close the review holes without Tab replacement. |
| 3 | Emacs motion keep-plain-in-sync | Needed once wrap paints styled `READLINE_LINE`; Left/Right/Home desync `_MBX_HIGHLIGHT_POINT` while `_MBX_HIGHLIGHT_ACTIVE=1` skips recapture. After slice 1. |
| — | `HLT-003` hostile corpus + highlight p99 | After wrap evidence. Do not idle product work on percentiles (`docs/latency-budget-deferral.md`). |
| — | Dim paint, `MBX_GHOST=1`+`MBX_HIGHLIGHT=1`, type-to-filter Ctrl+R | Deferred. |
| — | Lexer `#` word-start accuracy, UTF-8 `strip_to_plain` | Notes, not this close. |

## Goal (slice 1 — highlighting actually installs)

1. Replace `_mbx_highlight_bind_x` occupancy with the ghost rule
   (`_mbx_ghost_can_wrap`): refuse a competing `bind -x` unless
   `MBX_HIGHLIGHT_OVERRIDE=1`; **allow** wrapping stock `self-insert`,
   `backward-delete-char`, and helper `accept-line` / `delete-char` keys.
   Do not treat `bind -p` presence alone as occupied.
2. Set `_MBX_HIGHLIGHT_BOUND=1` only when `bind -X` lists
   `_mbx_highlight_self_insert` for a printable and the Enter path is actually
   armed (see Enter rules). Bind helpers **before** printables (M-044).
3. Gate helper output by **strip-then-compare**, not
   `_mbx_text_has_c0_or_del` on the styled string. Strip markers and SGR;
   remainder must equal `_MBX_HIGHLIGHT_PLAIN`; refuse any other C0/DEL in
   the remainder. Only then assign `READLINE_LINE` / `READLINE_POINT`.
4. Suspend `monitor`/`notify` around the highlight process substitution and
   restore on every return (M-049). Save helper stdout before
   `_mbx_wait_child_until` overwrites `REPLY`.
5. **Enter (M-041, non-negotiable):** do not wrap `\C-m` / `\C-j` as `bind -x`
   that then runs `bind "\"$keyseq\""` hoping to invoke `accept-line`. That
   pattern is unproven and matches the ghost failure. Copy the ghost shape:
   `\C-m` / `\C-j` stay **Readline functions**, not `bind -x`.
   - Keep `_MBX_HIGHLIGHT_PLAIN` as execution truth.
   - Arm a Readline-only accept macro that does **not** `eval` the line.
   - Slice 1 must PTY-prove **one** Enter after a styled (or attempted-style)
     line runs the **plain** bytes and the next prompt is usable.
   - If a Readline-only restore is impossible while `READLINE_LINE` holds
     markers, **stop and reassess**. Amend ADR 0013 in that same change. Do
     not `eval`, TIOCSTI, or “press Enter twice”.
6. Do not mark `HLT-002` complete until H-1–H-6 pass. Do not start overlay
   leftovers (rank 2) in the same implement pass.

## Goal (slice 2 — overlay review leftovers)

Do not start until slice 1 is handed off or the user asks for overlay only.

1. Cap `_MBX_COMP_OVERLAY_CANDIDATES` (and kinds/descs) at 8. Cycle/accept
   only that window.
2. Sanitize displayed candidate **bytes** with the same C0/`$`/\`/`\` rules as
   descriptions before writing the tty. Insertion still uses ranked-accept
   word replace (M-039).
3. Do not bind stock `\C-g` `abort`. Pick a free dismiss chord after `bind -p`
   (M-040). Default candidate: `\C-x\C-g` is also `abort` — inspect stock
   emacs; prefer an unbound `\C-x` letter (not `\C-xg` glob-list, not
   `\C-xh` search). Record the chosen sequence in ADR 0013. Toggle-off via
   `\C-x\C-o` may remain the hide path if dismiss cannot bind.
4. Draw: either implement DECSC/DECRC as ADR 0013 says, **or** amend the ADR
   to the tty-line list and prove clear does not erase `PS1`. Cursor-up
   `\033[A\033[2K` from the prompt line is not acceptable. `_mbx_comp_overlay_have_tty`
   must match the fd actually written (`/dev/tty` or the ADR-chosen stream).
5. PTY: wait for an overlay-only marker (selected `> aaflag` plus unchanged
   `mbx_comp_rank aa`), not a bare `aaflag` substring (M-038). Assert
   `_MBX_COMP_OVERLAY_BOUND=1` and `bind -X` contains `_mbx_comp_overlay_toggle`.

## Out of scope (hard)

- Dim after-every-key ANSI paint
- Combining `MBX_GHOST=1` and `MBX_HIGHLIGHT=1`
- Rebinding Tab, stock `\C-r`, stock `\C-g` `abort`
- `eval` / executing `READLINE_LINE` from `bind -x`
- `set -euo pipefail` in sourced modules
- Type-to-filter Ctrl+R overlay
- `HLT-003` percentile benches; macOS `HRD-001`; `GIT-005`
- Marking `HLT-002`, `HLT-003`, `COMP-004`, Phase 6, or `G5` revisit complete
- Lexer comment-at-word-start or UTF-8 `strip_to_plain` rewrite
- Committing unless asked (implementers: follow the user/cloud commit policy)

## Asserts (slice 1)

| ID | Evidence |
| --- | --- |
| H-1 | PTY `MBX_HIGHLIGHT=1`: `_MBX_HIGHLIGHT_BOUND=1` **and** `bind -X` contains `_mbx_highlight_self_insert` |
| H-2 | Module: styled stub with SOH/ESC/STX is accepted only when strip equals `_MBX_HIGHLIGHT_PLAIN`; a stub that injects extra ESC or differs after strip leaves the plain buffer and does not set `_MBX_HIGHLIGHT_ACTIVE=1` |
| H-3 | PTY: type `printf 'HL:plain\n'` under highlight; **one** Enter prints `HL:plain`; executed bytes are not SGR/SOH; next prompt accepts `echo ok` |
| H-4 | `MBX_HIGHLIGHT` unset: no `_mbx_highlight_self_insert` in `bind -X`; typing Enter is stock |
| H-5 | Module: highlight helper spawn saves/restores `$-` monitor/notify (same contract as `_mbx_search_restore_jobs`) |
| H-6 | Occupied printable `bind -x` is not overwritten unless `MBX_HIGHLIGHT_OVERRIDE=1`; `_MBX_HIGHLIGHT_BOUND` stays `0` if wrap cannot arm Enter |

## Asserts (slice 2)

| ID | Evidence |
| --- | --- |
| O-1 | Module: ranked list of 10 rows snapshots exactly 8 overlay candidates; cycle never selects index 8 |
| O-2 | Module: candidate containing ESC/`$` is displayed sanitized; ranked-accept still inserts the raw eligible token when the word is a prefix at the snap offset |
| O-3 | PTY: `_MBX_COMP_OVERLAY_BOUND=1` and `bind -X` contains `_mbx_comp_overlay_toggle`; stock `bind -p` `"\C-g": abort` remains |
| O-4 | PTY: Tab then overlay chord shows `> aaflag` (or the documented selected prefix) while the input line stays `mbx_comp_rank aa`; second chord hides the list; prompt still usable |
| O-5 | Overlay clear/refresh does not delete the `> ` prompt in the same PTY recording |

## Asserts (slice 3 — after slice 1)

| ID | Evidence |
| --- | --- |
| M-1 | PTY: type `echo ab`, Left, type `X`; executed line is `echo aXb` (plain bytes), not a dropped or duplicated character |

## Docs to update (each slice, in the same change)

- This file’s status and remaining ranks
- `docs/hlt-002-integration-plan.md` — stay `validation` until H-1–H-6
- `docs/comp-004-overlay-plan.md` — stay `validation` until O-1–O-5; must not say `complete`
- `docs/adr/0013-opt-in-continuous-decoration.md` — wrap rule, Enter/M-041, strip-then-compare, overlay dismiss chord and draw
- `docs/roadmap.md` — immediate next work; do not mark Phase 6 complete
- `docs/architecture.md` — drop the leftover sentence that both denies and claims live highlighting
- `MISTAKES.md` — add confirmed entries listed below when the fix lands (one writer)

## MISTAKES entries to add when the fix lands

Do not add speculative rows before the code change. When slice 1/2 edits are
authorized, record:

- Highlight install reported `_MBX_HIGHLIGHT_BOUND=1` with no `bind -X` widgets
  (M-040 recurrence). Prevention: H-1.
- `_mbx_text_has_c0_or_del` on styled helper output rejected every highlighted
  line. Prevention: H-2.
- `index+=2` on a non-integer concatenated (`5`+`2`→`52`). Prevention: only
  `index=$((index + n))` in sourced modules.
- `_mbx_wait_child_until` clobbering `REPLY` after highlight read (same as
  search-root). Prevention: save payload before wait.

## Validate

```bash
bash tests/bash/modules.bash
cargo test -p mbx highlight::
cargo test -p mbx-pty --test highlight -- --nocapture
cargo test -p mbx-pty --test completion_harness overlay_lists ranked_accept_works_with_overlay_env -- --nocapture
bash tests/run.bash
```

## Stop

Do not start rank 2 overlay leftovers in the same implement pass as slice 1.
Do not start rank 3 motion until H-1–H-6 exist. Do not start dim paint,
ghost+highlight composition, or `HLT-003` percentiles. Do not mark `HLT-002`
or `COMP-004` complete.

## Composer packet (slice 1 only)

Copy this XML as the implement TODO. Rank 2+ stay out of the packet.

```xml
<composer_packet model="composer-2.5">
  <ranking why="do not pick overlay leftovers until highlight wrap is evidenced">
    <item rank="1" status="implement_now" id="HLT-002">Wrap actually installs; strip-then-compare; M-049 jobs; M-041 Enter; H-1–H-6. Why this: review proved highlighting is a no-op.</item>
    <item rank="2" status="blocked_or_later" id="COMP-004">Overlay cap/sanitize/dismiss/draw. After H-1–H-6 or a dedicated overlay ask.</item>
    <item rank="3" status="blocked_or_later" id="HLT-motion">Left/Right/Home plain-buffer sync. After wrap paints styled READLINE_LINE.</item>
  </ranking>
  <composer_task id="hlt-002-wrap-close">
    <follow>docs/hlt-comp-review-close-plan.md slice 1. Do not invent extra cases.</follow>
    <hard_rules>
      Do not mark HLT-002, HLT-003, COMP-004, or Phase 6 complete.
      Do not eval READLINE_LINE. Do not bind stock abort. Do not start overlay leftovers.
      Do not commit unless asked.
    </hard_rules>
    <bootstrap order="required">
      <step>Read MISTAKES.md in full (M-038, M-040, M-041, M-044, M-049, M-050).</step>
      <step>Read docs/hlt-comp-review-close-plan.md, ADR 0013, ADR 0010, docs/roadmap.md.</step>
      <step>Read bash/highlight.bash, bash/ghost.bash (_mbx_ghost_can_wrap, _mbx_ghost_arm_enter), bash/search.bash job restore.</step>
      <step>git status --short. Preserve unrelated work.</step>
    </bootstrap>
    <goal>H-1 through H-6 have durable module and PTY evidence. bind -X shows highlight widgets. One Enter runs plain bytes.</goal>
    <implement>
      <item>Rewrite highlight wrap occupancy to match _mbx_ghost_can_wrap. BOUND=1 only with widgets plus armed Enter.</item>
      <item>Bind Enter/delete helpers before printables (M-044).</item>
      <item>Strip-then-compare helper output to _MBX_HIGHLIGHT_PLAIN; refuse extra C0.</item>
      <item>Monitor/notify save-restore around the helper; save styled payload before _mbx_wait_child_until.</item>
      <item>Enter: Readline-only path (M-041). If that cannot restore plain, stop and amend ADR 0013. No eval.</item>
      <item>PTY H-1 and H-3 must fail if wrap is absent (assert bind -X and one-Enter plain execution).</item>
    </implement>
    <docs>
      <file>docs/hlt-comp-review-close-plan.md</file>
      <file>docs/hlt-002-integration-plan.md</file>
      <file>docs/adr/0013-opt-in-continuous-decoration.md</file>
      <file>docs/roadmap.md</file>
      <file>docs/architecture.md</file>
      <file>MISTAKES.md</file>
    </docs>
    <validate>
      <cmd>bash tests/bash/modules.bash</cmd>
      <cmd>cargo test -p mbx-pty --test highlight -- --nocapture</cmd>
      <cmd>bash tests/run.bash</cmd>
    </validate>
    <review required="true">
      Re-read this plan and the diff. Confirm bind -X after MBX_HIGHLIGHT=1 lists
      _mbx_highlight_self_insert. Confirm H-3 is not satisfied by stock Enter.
      Fix defects, missed asserts, stale docs, and MISTAKES.md gaps. Do not start rank 2.
    </review>
    <stop>Do not start overlay leftovers or motion wrap. Do not commit unless asked.</stop>
  </composer_task>
</composer_packet>
```
