# GHST-001 slice: MBX2 QUERY/RESULT/CANCEL wire

Status: `complete` for the wire slice (2026-08-20). Ghost stale rejection is
recorded in `docs/ghst-001-ghost-query-plan.md`.

## Why this slice

ADR 0011 accepted generation-tagged QUERY on MBX2. This slice lands exact field
layouts, Bash encode/decode, helper QUERY via `HistorySearch`, and CANCEL→ACK.
Ghost still uses sync CLI search until the next leftover.

| Rank | Item | Why not now |
| --- | --- | --- |
| 1 | QUERY wire (this plan) | Unblocks async ghost without decoration. |
| 2 | Ghost generation + stale rejection | Next slice after wire tests pass. |
| — | Dim paint / overlay | Still needs continuous decoration. |

## Wire layout (exact)

```text
MBX2<TAB>id<TAB>QUERY<TAB>generation<TAB>mode<TAB>text-or-<TAB>limit
# mode ∈ prefix|fuzzy|cwd|repo|branch|failed|recent
# text is "-" when mode is failed or recent; otherwise the needle/path/root/branch
# generation and limit are decimal (limit capped at MAX_QUERY_LIMIT)

MBX2<TAB>id<TAB>CANCEL<TAB>generation

MBX2<TAB>id<TAB>RESULT<TAB>generation<TAB>count<TAB>cmd1…cmdN
# exact field count = 5 + count; each cmd percent-escaped; count may be 0

MBX2<TAB>id<TAB>ACK
MBX2<TAB>id<TAB>ERROR<TAB>kind
```

One RESULT frame per QUERY (keeps `_mbx_engine_exchange` 1:1).

## Goal

1. Document layouts in `docs/protocol-mbx2.md`.
2. Bash encode QUERY/CANCEL and decode RESULT/ERROR with exact field counts.
3. `HistoryService` answers QUERY via `HistorySearch`; CANCEL → ACK.
4. Focused Rust + Bash module tests (hostile bytes, wrong counts, generation).
5. Do **not** change `bash/ghost.bash` in this slice.

## Out of scope (hard)

- Ghost async install / generation stale rejection / PTY
- Multi-frame RESULT streaming
- MBX1 changes, dim paint, marking `GHST-001` complete

## Validate

```bash
cargo test -p mbx --lib history_service
cargo test -p mbx --lib transport
bash tests/bash/modules.bash
bash tests/run.bash
```

## Stop

Do not wire ghost onto QUERY until the next named slice.
