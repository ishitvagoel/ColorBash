# GHST-001 slice: MBX2 async query ADR acceptance

Status: `complete` for the decision gate (2026-08-20). Implementation remains.
Do **not** mark `GHST-001` or `GHST-004` complete.

## Why this slice

Ghost Strategy A (ADR 0010) is on main. Sync per-keystroke search cannot cancel
stale work. Roadmap listed `GHST-001` as `blocked` on an async IPC ADR.
ADR 0011 accepts MBX2 QUERY/RESULT/CANCEL with generation-tagged stale
rejection.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | Async IPC ADR (this plan) | Named blocker for `GHST-001`. |
| 2 | Wire layout + helper handler | Next implementation slice after ADR. |
| — | Dim paint / overlay | Still needs continuous decoration. |

## Goal

1. Accept ADR 0011.
2. Move `GHST-001` from `blocked` to `ready`.
3. Do not implement QUERY framing or change `bash/ghost.bash` in this slice.

## Out of scope (hard)

- Protocol field layout code or PTY tests
- Dim ANSI, highlighting, overlay
- Changing MBX1
- Marking `GHST-001` / Phase 4 complete

## Stop

Do not start the wire implementation in this slice unless a follow-up packet
names it.
