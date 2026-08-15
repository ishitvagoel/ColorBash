# MBX2 protocol — history record ingestion

Status: implemented for the Phase 3A RECORD slice (`HIST-007`); `G2` evidence
(100k-row budgets, contention, and `.bash_history` invariance) remains.

## Purpose

MBX2 carries history records from the Bash recorder to the helper's writer port.
It is intentionally separate from MBX1: MBX1 stays prompt-oriented with additive
flag bits, while MBX2 carries typed, metadata-bearing records that MBX1 must
never silently extend to carry (ADR 0005 section 9).

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

- `RECORD` is the only data message in the Phase 3A slice; search remains a
  direct CLI operation and does not travel over the wire.
- `ERROR` carries a typed kind (`invalid`, `queue_full`, `storage`) and never
  command text.
- Fields are already folded Bash-normalized text from the admission authority;
  the helper treats them as inert data and applies ADR 0005 drop rules.

## Bounds and safety

- Bash enqueues with its own bounded deadline; a full queue or expired deadline
  drops the record with a command-text-free diagnostic counter.
- The helper validates the entry (empty/NUL/invalid-UTF-8/oversized), applies
  exclusions, and hands it to the bounded writer queue; `ACK` confirms enqueue,
  not commit.
- `PING`/`PONG` reuses the existing handshake semantics for liveness.

## Versioning

The magic `MBX2` is fixed for this slice. Adding request kinds, generation IDs,
cancellation, or stale-response rejection for interactive features is a later
MBX2 revision, not an MBX1 change.
