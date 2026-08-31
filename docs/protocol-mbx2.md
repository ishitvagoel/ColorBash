# MBX2 protocol — history record ingestion and feature queries

Status: RECORD slice implemented (`HIST-007`). `G2` is complete. Write-ack
percentiles are `deferred` (`docs/history-g2-write-ack-deferral.md`).
QUERY/RESULT/CANCEL wire layouts are specified for `GHST-001`
(`docs/adr/0011-async-feature-ipc.md`; `docs/ghst-001-query-wire-plan.md`).
Helper QUERY handling, Bash encode/decode, and ghost coprocess QUERY with
generation checks and overlapping delayed-RESULT skip are recorded
(`docs/ghst-001-ghost-query-plan.md`). Sync CLI search remains available.

Foreign-user open is recorded. Invariance, admission-parity, hostile inertness,
100k query p95, concurrent-writer contention, write-ack correctness, WAL
crash/corrupt recovery, WAL/SHM `0600` never-more-permissive, many-match prefix
covering-index, writer idle-flush for live reader visibility, and 100k-row
v1→v2 migration evidence are recorded.

`HIGHLIGHT`/`STYLED` (ADR 0014) extend MBX2 with the same generation and
stale-skip discipline for opt-in syntax highlighting, dispatched by an
independent `HighlightHandler` so `MBX_HIGHLIGHT=1` does not require
`MBX_HISTORY=1`. `color` is `_mbx_highlight_color_flag` (ADR 0015);
styled bytes paint on a reserved preview row rather than `READLINE_LINE`
(`M-064` fixed).

## Purpose

MBX2 carries history records from the Bash recorder to the helper's writer port,
and interactive feature queries (generation-tagged search) on the same framing.
It is intentionally separate from MBX1: MBX1 stays prompt-oriented with additive
flag bits, while MBX2 carries typed, metadata-bearing records and queries that
MBX1 must never silently extend to carry (ADR 0005 section 9; ADR 0011).

## Framing

Identical transport rules to MBX1 so the existing coprocess/socket adapters and
their bounds apply unchanged:

- One UTF-8 message per line; fields tab-separated; `%` and control bytes
  percent-escaped (upper/lower hex accepted); NUL rejected after decoding.
- Payload at most 65,536 bytes; LF/CRLF/EOF terminate a frame without counting
  toward the payload limit.
- The Bash client binds the request ID and validates magic, ID, kind, and field
  count before accepting a response.

## Messages

### RECORD / PING (Phase 3A)

Requests:

```text
MBX2<TAB>request-id<TAB>PING
MBX2<TAB>request-id<TAB>RECORD<TAB>session-id<TAB>sequence<TAB>history-number-or-<TAB>
    command-text<TAB>cwd<TAB>completed-at<TAB>status<TAB>duration-ms-or-<TAB>host<TAB>user
```

Responses:

```text
MBX2<TAB>request-id<TAB>PONG
MBX2<TAB>request-id<TAB>ACK
MBX2<TAB>request-id<TAB>ERROR<TAB>kind
```

- `RECORD` is the ingest message; CLI search also remains a direct operation.
- Repository root/branch are **not** RECORD fields. The helper writer may
  enrich stored rows from `start_cwd` after ACK (`HIST-010`); the wire layout
  stays ten data fields.
- `history-number` is the list number from `history 1` for the newest admitted
  entry, or `-` when absent. It is diagnostic only and is not `HISTCMD`.
- `ERROR` carries a typed kind (`invalid`, `queue_full`, `storage`) and never
  command text.
- Fields are already folded Bash-normalized text from the admission authority;
  the helper treats them as inert data and applies ADR 0005 drop rules.

### QUERY / CANCEL (ADR 0011 / GHST-001)

Requests:

```text
MBX2<TAB>request-id<TAB>QUERY<TAB>generation<TAB>mode<TAB>text-or-<TAB>limit
MBX2<TAB>request-id<TAB>CANCEL<TAB>generation
```

- `generation` is a decimal u64 chosen by the client; every RESULT echoes it.
- `mode` is one of `prefix`, `fuzzy`, `cwd`, `repo`, `branch`, `failed`,
  `recent` (same semantics as `mbx history search …`).
- `text` is the needle, path, repo root, or branch name. For `failed` and
  `recent` it must be the literal `-`.
- `limit` is a decimal usize; the helper caps it at `MAX_QUERY_LIMIT` (500).
- `CANCEL` is best-effort. The helper may still emit a late RESULT for that
  generation; Bash **must** ignore stale generations (client duty).

Responses:

```text
MBX2<TAB>request-id<TAB>RESULT<TAB>generation<TAB>count<TAB>cmd1…cmdN
MBX2<TAB>request-id<TAB>ACK
MBX2<TAB>request-id<TAB>ERROR<TAB>kind
```

- One `RESULT` frame answers one `QUERY` (keeps the exchange 1:1). Exact field
  count is `5 + count`. `count` may be `0`. Each `cmd` is percent-escaped
  ordinary command text and must never be executed by the helper.
- If encoding all candidates would exceed the 64 KiB payload bound, the helper
  drops trailing candidates and reduces `count` so the frame fits.
- `CANCEL` responds with `ACK` (stub cancel tracking is allowed).
- Typed ERROR kinds include `invalid`, `unsupported`, `unsupported query mode`,
  and storage kinds; never command text.

### HIGHLIGHT / STYLED (ADR 0014)

Request:

```text
MBX2<TAB>request-id<TAB>HIGHLIGHT<TAB>generation<TAB>color<TAB>point<TAB>text
```

- `generation` is a decimal u64 chosen by the client; every STYLED echoes it,
  with the same stale-generation-skip contract as `QUERY`/`RESULT` (ADR 0011):
  an older generation is dropped, a newer one fails the request.
- `color` is `0` or `1`. It exists so the caller — the only side that can see
  the real terminal — decides colorability; the helper never infers it from
  its own stdout (`M-062`). Bash passes `_mbx_highlight_color_flag`
  (ADR 0015; `bind -x` stdout is often a pipe). Any other value is
  `ERROR invalid`.
- `point` is the plain-buffer cursor position as a Unicode scalar count
  (decimal usize), matching Bash `READLINE_POINT` / `${#var}` (ADR 0015).
- `text` is the plain command text, percent-escaped; bounded to a few KiB by
  the same limit `mbx highlight` enforces on the CLI.

Response:

```text
MBX2<TAB>request-id<TAB>STYLED<TAB>generation<TAB>point<TAB>line
MBX2<TAB>request-id<TAB>ERROR<TAB>kind
```

- `point` is the styled-buffer cursor as a Unicode scalar count; `line` is the
  styled (or, at `color=0`, plain) text, percent-escaped. One `STYLED` frame
  answers one `HIGHLIGHT` request. The interactive preview ignores the
  returned point; the cursor stays on the plain `READLINE_LINE` (ADR 0015).
- `HIGHLIGHT` has no `CANCEL`: unlike ghost's background QUERY, a highlight
  request is synchronous from Bash's perspective (the keystroke handler
  blocks on it up to `MBX_HIGHLIGHT_TIMEOUT`), so there is no in-flight
  request to cancel — the generation-skip rule alone is enough to discard a
  reply that arrives after a newer keystroke superseded it.
- `HIGHLIGHT` is dispatched by an independent `HighlightHandler`, never by
  `HistoryHandler`; it is present regardless of `MBX_HISTORY`, and returns
  `ERROR unsupported` only if the composition root did not wire a
  `HighlightHandler` at all.

## Bounds and safety

- Bash enqueues RECORD with its own bounded deadline; a full queue or expired
  deadline drops the record with a command-text-free diagnostic counter.
- The helper validates the entry (empty/NUL/invalid-UTF-8/oversized), applies
  exclusions, and hands it to the bounded writer queue; `ACK` confirms enqueue,
  not commit.
- QUERY work is bounded by the same class of search budgets as the CLI. Head-of-
  line blocking on the sequential coprocess is accepted until measured evidence
  requires a dedicated feature FD (new ADR).
- `PING`/`PONG` reuses the existing handshake semantics for liveness.

## Versioning

The magic `MBX2` is fixed. RECORD, QUERY, and HIGHLIGHT share framing. Exact
layouts above are the contract for the `GHST-001` wire slice and, for
HIGHLIGHT/STYLED, ADR 0014. Ghost coprocess QUERY with generation checks and
overlapping delayed-RESULT skip are recorded. Not an MBX1 change.
