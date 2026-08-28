# HRD-004: Install, upgrade, disable, removal, crash, and recovery

Status: `complete` (2026-08-26). `HRD-002` is complete. This packet records
lifecycle evidence for a development install that never writes shell startup
files. It does **not** close `G5` or `HRD-001`. Overlay, highlighting, dim
paint, and percentile benches stay `deferred`.

## Why this slice

Immediate next work after `HRD-002`. Packaging a distro installer is still
out of deferred-scope (no cloud/graphical installer). The repo already has a
dev setup script, an interactive loader that must not edit `~/.bashrc`,
disable/fallback paths, helper-crash PTY, and WAL crash recovery. Those were
never inventoried as `HRD-004`.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | `HRD-004` lifecycle evidence (this plan) | Runnable on this Linux host. |
| — | `HRD-001` macOS pairwise matrix | Needs a macOS host. |
| — | `HRD-003` release latency | Percentiles `deferred`. |
| — | Distro packages / Homebrew formula | Deferred product-scope; not this MVP. |

## Goal

1. Map each named lifecycle claim to tests. Add an assert only when a named
   row has no evidence.
2. Prove `source bash/init.bash` and `scripts/dev-setup.bash` do not modify
   `~/.bashrc`.
3. Document disable and removal as "stop sourcing / unset opt-in flags".
4. Move `HRD-004` to `complete`. Do not mark `G5` or `HRD-001` complete.

## Out of scope (hard)

- Overlay, highlighting, dim paint
- macOS matrix (`HRD-001`)
- Percentile benches (`HRD-003`)
- Writing or rewriting `~/.bashrc` from product code
- A package-manager installer
- `set -euo pipefail` or `MBX_DBG` in sourced modules
- Marking `G5` complete

## Evidence inventory

| ID | Claim | Evidence | Status |
| --- | --- | --- | --- |
| L-1 | Dev setup builds the helper and does not edit `~/.bashrc` | `scripts/dev-setup.bash` prints a source line; smoke `SETUP_NO_BASHRC` | satisfied |
| L-2 | Interactive `source init.bash` does not write `~/.bashrc` | smoke isolated-`HOME` sentinel (L-2) | satisfied |
| L-3 | Missing helper / helper crash stay usable | smoke fallback + recovery; PTY `missing_helper_falls_back_to_usable_prompt`, `helper_crash_degrades_without_disabling_the_shell` | satisfied |
| L-4 | Disable renderer / history-off | `MBX_DISABLE_RENDERER`; `MBX_HISTORY` unset creates no store (M-024) | satisfied |
| L-5 | Removal is stop sourcing | non-interactive init is a no-op; README disable/remove | satisfied |
| L-6 | Upgrade / store recovery | schema migrate 100k v1→v2; WAL/SHM corrupt tests; idle-flush | satisfied |
| leftover | Signed packages, Homebrew, macOS codesign | post-MVP packaging | deferred — does not block this close |

## Asserts added this slice

| ID | Evidence |
| --- | --- |
| L-2 | Isolated `HOME` with a sentinel `.bashrc`; `source bash/init.bash`; file bytes unchanged |
| L-1 | `scripts/dev-setup.bash` contains `does not modify ~/.bashrc` and no `>>` / `bashrc` write |

## Docs to update

1. `docs/roadmap.md` — `HRD-004` `not-started` → `complete`; Immediate next
   work is `HRD-001` host-blocked leftover / do not fake macOS; changelog.
2. `README.md` — disable and remove without an installer.
3. This file — Status `complete` after the roadmap edit.

## Remaining

`HRD-001` / `G5` stay host-blocked. `HRD-003` stays `not-started` with
percentiles `deferred`. `scripts/dev-setup.bash` and `source bash/init.bash`
still never write `~/.bashrc`. `scripts/install.bash` writes
`~/.config/mbx/config.bash` for a chosen profile; `--bashrc` is an explicit
opt-in that writes a managed block and is not the default loader.
