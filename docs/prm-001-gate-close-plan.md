# PRM-001 gate close

Status: `complete` (2026-08-16).

Close `PRM-001` (`validation` → `complete`) from inventory of existing
prompt-segment evidence plus the missing nerd-icon exact-byte fixture. This
is not a new prompt feature.

## Why this slice

`G0` prompt requirements are already `complete`. `PRM-001` stayed in
`validation` because the named icon leftover had no exact-glyph assert:
`FLAG_NERD_ICONS` selected glyphs in `crates/cli/src/prompt.rs` without a
fixture locking SSH / path / git / failed-exit / production / arrow bytes.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `PRM-001` gate close (this plan) | Named leftover; host can produce exact-byte evidence. |
| 2 | `HIST-010` / `GIT-003` | Separate leftover; do not duplicate open PRs. |
| — | Overlay / ghost / highlighting | Unproven continuous decoration (ADR 0003 / B-5). |
| — | `COMP-004` popup | Stays `discovery`. |
| — | `PRM-006` duration-policy close | Separate leftover; still `validation` on `main`. |
| — | `HRD-001` / `G5` | Needs a macOS host. |

## Goal

1. Inventory existing path / Git / status / duration / SSH / production /
   theme evidence. Do not invent overlay, fallback-nerd glyphs, or a second
   renderer.
2. Add exact-byte native nerd-icon fixtures for the locked substitutions.
   ASCII wins when both icon flags are set. Last-resort fallback stays
   font-safe ASCII even when the nerd flag is present.
3. Move `PRM-001` to `complete`. Do not mark `COMP-004`, overlay, or
   `PRM-006` complete.

## Out of scope (hard)

- Overlay, ghost text, syntax highlighting, enhanced Ctrl+R
- Teaching the process-free fallback Nerd Font glyphs
- A second prompt renderer (`PRM-009`)
- Marking `COMP-004` or `PRM-006` complete
- macOS / `G5` / `HRD-001`
- `set -euo pipefail` in sourced Bash
- Committing unless asked

## Evidence inventory

| Segment | Existing evidence | This close |
| --- | --- | --- |
| Path | `plain_prompt_preserves_segment_order`; `injected_home_controls_path_compaction`; display-width compaction tests | unchanged |
| Git | provider substitute + sanitizer; omit on error; git-disabled does not invoke provider | unchanged |
| Status | nonzero `exit N` in native and fallback | nerd ` N` exact bytes |
| Duration | `duration_boundaries_have_stable_formatting`; native/fallback ≥ 2 s | unchanged |
| SSH | fallback SSH-only sanitizer; native production-over-SSH | native SSH-only ASCII; nerd `󰒍 host` |
| Production | native and fallback production-over-SSH | nerd `󰀪 PROD` exact bytes |
| Theme | 16 / 256 / truecolor / `NO_COLOR` native and fallback | unchanged |
| Icons | ASCII labels; CLI last-icon-option; flags from `MBX_ICONS=nerd` | native exact glyphs; ASCII wins over nerd; fallback stays ASCII |

Additional durable evidence already on `main`:

- `tests/bash/modules.bash`: `PS1` is not written by adapters; production
  precedence; SSH sanitizer; hostile C0/DEL corpus.
- `docs/architecture.md` prompt-path ownership; `docs/ux-spec.md` G0 visual
  contract; `docs/adr/0002-rust-helper-architecture.md`.

## This close

- `nerd_icons_use_locked_glyphs_for_ssh_git_path_and_failed_exit`
- `nerd_icons_use_locked_production_glyph`
- `ascii_icons_win_over_nerd_icons`
- `ssh_context_renders_without_production`
- fallback nerd-flag stays font-safe ASCII in `tests/bash/modules.bash`

Expected native nerd bytes (plain color, host `box`, cwd `/tmp/project`,
clean `main`, status `1`):

```text
󰒍 box   /tmp/project  󰊢 main   1\n❯ 
```

The `\\n` is the PS1 line separator, not a raw newline. Two spaces separate
segments. Do not invent `` or product-brief two-part git icons; assert the
implemented `󰊢 ` + branch form.

## Docs

- `docs/roadmap.md`: `PRM-001` → `complete`; changelog; immediate next stays
  `HIST-010` / overlay blocked / `COMP-004` discovery / `PRM-006` validation.
- `docs/architecture.md`: nerd exact-byte pointer; fallback stays ASCII.
- `docs/ux-spec.md`: native nerd substitution table.
- `README.md`: `MBX_ICONS=nerd` on the tryable prompt row.
