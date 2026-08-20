# GIT-004 slice: structured Git completion kinds

Status: `complete` (2026-08-16). `COMP-003` kinds/ranking and ranked-accept
exist. This packet classifies wrapped Git (and the `mbx_comp_git` fixture)
candidates as `ref`, `flag`, or `file` beside `COMPREPLY`. No Git subprocess.
Tab bytes stay stock. Do not wrap real `git` on default install (M-037).

## Goal

1. When `COMP_WORDS[0]` is `git` or `mbx_comp_git`, kinds are `flag` for
   `-*`, `file` for values containing `/`, else `ref`.
2. `COMPREPLY` order and insertion bytes stay stock. Ranking remains additive.
3. Fixture `mbx_comp_git` installs only when `MBX_COMP_FIXTURES=1`.
4. Default install still does not wrap stock `git`. Users may
   `_mbx_comp_wrap_existing_f git` opt-in.
5. `GIT-004` may move to `complete`. Do not start overlay or `GIT-005`.

## Remaining

Overlay stays unproven. `GIT-003` / `HIST-010` repository root/branch on history
rows is complete (`docs/hist-010-git-003-plan.md`).
`GIT-005` stays post-MVP.
