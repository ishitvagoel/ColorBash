# ADR 0011: Async feature queries on MBX2 with generation IDs

Status: Accepted (2026-08-20)

## Context

Opt-in inline ghost (ADR 0010) already shows a history suffix using a
**synchronous**, timeout-bounded `mbx history search` fork from Bash
`bind -x`. That path is correct for MVP text, but it can still block a
keystroke for the full search budget and cannot cancel an in-flight lookup
when the typed prefix changes.

`GHST-001` needs asynchronous ranked lookup with generation IDs, cancellation,
and stale-result rejection. ADR 0004 keeps the Bash coprocess as the default
transport. ADR 0005 / `docs/protocol-mbx2.md` already say interactive query
kinds, generation IDs, and cancellation are a later **MBX2** revision, not an
MBX1 change. MBX1 remains prompt-oriented with additive flag bits.

## Decision

1. **Extend MBX2**, not MBX1, for interactive feature queries. Framing,
   percent-escaping, 64-KiB bounds, and correlation IDs stay identical to
   MBX1/MBX2 RECORD.
2. Add typed request/response kinds on the same coprocess (and socket) path:
   - `QUERY` — bounded history search (prefix/fuzzy/cwd/repo/branch/failed as
     already exposed by the CLI). Fields include a monotonic **generation**
     (u64 decimal), search mode, query text, optional filters, and limit.
   - `RESULT` — zero or more candidate rows for that generation, then a
     terminal `RESULT_END` (or a single multi-row frame if a later slice
     proves one frame is enough). Candidate text is ordinary command bytes;
     never execute.
   - `CANCEL` — best-effort cancel of an in-flight generation; helper may
     still emit a late `RESULT` that Bash **must** ignore.
   - `ERROR` — typed kind only; never command text (same privacy rule as
     RECORD).
3. **Stale rejection is a Bash/client duty.** The helper tags every `RESULT`
   with the generation from the matching `QUERY`. Ghost (and later search)
   keep a single “current generation” and discard any result whose generation
   is less than current. Do not apply results after the typed prefix no longer
   matches.
4. **Sync CLI search remains.** Direct `mbx history search …` stays the
   development and fallback path. Ghost may keep sync lookup until `GHST-001`
   lands; ADR 0010 Strategy A is not revoked.
5. **Head-of-line policy.** One sequential coprocess can block behind a slow
   QUERY. Bound helper work with the same class of deadlines as history
   search budgets. If measured HOL blocking violates keystroke budgets,
   revisit a dedicated feature FD or socket in a **new** ADR; do not fork a
   second helper by default in this decision.
6. **Privacy.** No command text in logs, traces, or ERROR payloads (M-023).
   QUERY text is protocol data only.

## Alternatives

- **New MBX3 magic:** rejected for now; MBX2 already reserved this extension
  and shares framing with RECORD.
- **Per-keystroke process spawn only:** already used by ghost sync; cannot
  cancel mid-flight and pays startup every key.
- **Unix daemon fan-out:** measured faster for PING, but lifecycle complexity
  is out of scope until HOL evidence requires it (ADR 0004).
- **Put queries on MBX1:** rejected; would overload prompt flags and break
  the RECORD/prompt separation in ADR 0005.

## Consequences

- `GHST-001` moves from `blocked` to `ready` for implementation planning.
- A protocol doc revision (`docs/protocol-mbx2.md` or a sibling) must specify
  exact field layouts before code lands.
- Bash ghost install stays tty-interactive and opt-in (`MBX_GHOST=1` +
  `MBX_HISTORY=1`). Failure or timeout still degrades to typed text only.
- Dim paint and continuous decoration remain blocked on ADR 0003; this ADR
  does not unlock highlighting or overlays.

## Validation plan

Before marking `GHST-001` complete:

- Cross-language encode/decode tests for QUERY/RESULT/CANCEL/ERROR, including
  hostile bytes and generation overflow bounds.
- PTY: type a short prefix, then a longer non-matching prefix before the
  first RESULT arrives; the stale generation must not change `READLINE_LINE`.
- CANCEL after QUERY does not disable the shell; a later usable prompt remains.
- No command text in helper diagnostics under `MBX_LOG=trace`.
