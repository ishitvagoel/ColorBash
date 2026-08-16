# MBX1 protocol

## Goals

MBX1 is a small local request/response protocol used identically over coprocess
stdio and Unix streams. It is versioned, line-delimited, bounded to 64 KiB, and
simple enough for Bash to produce and validate with builtins.

## Framing

Each UTF-8 message occupies one line. Fields are separated by a tab. Literal `%`,
control characters, tabs, and line breaks inside a field are percent-encoded as
uppercase UTF-8 bytes. Decoders accept upper- or lowercase hex and reject malformed
escapes. Unescaped line terminators are invalid. NUL is rejected even after
percent decoding because Bash variables cannot represent it.

The encoded message payload is at most 65,536 bytes; an LF or CRLF framing
delimiter is not part of that payload limit. A final EOF may delimit the last
message. Rust and Bash normalize EOF/LF/CRLF before applying the payload limit and
share `MAX-1`, `MAX`, and `MAX+1` boundary tests for every terminator. Reads are
capped at the payload plus the two-byte CRLF allowance. Bash reads under the C
locale in bounded chunks and treats raw NUL as an observable forbidden delimiter,
so it does not first allocate an arbitrary peer line. Prompt request encoding
preflights fixed framing overhead and escaped-field growth before producing a
line.

Requests:

```text
MBX1<TAB>request-id<TAB>PING
MBX1<TAB>request-id<TAB>PROMPT<TAB>cwd<TAB>status<TAB>duration-or--<TAB>flags
```

Responses:

```text
MBX1<TAB>request-id<TAB>PONG
MBX1<TAB>request-id<TAB>PROMPT<TAB>escaped-PS1
MBX1<TAB>request-id<TAB>ERROR<TAB>message
```

The Bash client accepts a response only when magic, request ID, response kind,
field count, and timeout all match. Unexpected input fails closed to the next
prompt transport/fallback.

## Prompt flags

| Bit | Value | Meaning |
| ---: | ---: | --- |
| 0 | 1 | no color |
| 1 | 2 | ASCII/text icons |
| 2 | 4 | Nerd Font icons explicitly enabled |
| 3 | 8 | SSH context |
| 4 | 16 | production context |
| 5 | 32 | Git lookup disabled |
| 6 | 64 | prefer 16-color ANSI SGR |
| 7 | 128 | prefer truecolor (`38;2`) SGR |

When bit 0 (`FLAG_NO_COLOR`) is set, bits 6 and 7 are ignored for rendering.
Otherwise `FLAG_TRUECOLOR` wins over `FLAG_COLOR_16`; when neither is set and
color is enabled, renderers use the default 256-color `38;5` palette.

Unknown bits must be ignored within MBX1 so additive capability flags remain
forward-compatible. Coprocess requests and Bash fallback carry the raw value;
per-call mode forwards that same integer through `mbx prompt --flags <u32>`.
Named CLI options applied later mutate only their known bits, preserving all
others.

## Security properties and limits

- Unix sockets are created with mode `0600` and never replace an existing path.
- Rust transport reads and writes enforce the 64-KiB payload limit before handing
  messages across the transport/application boundary. Transport owns response
  correlation IDs; a handler returns response content only.
- Bash bounds inbound acquisition and outbound construction, rejects raw or
  decoded NUL, and applies the one render deadline while encoding, reading,
  splitting, and percent-decoding.
- Git stdout uses a true 1-MiB-plus-one capped read and a maximum 50-ms refresh
  deadline. The normal timeout path attempts direct-child kill/reap and reports a
  typed cleanup failure if that cannot complete; descendant-tree termination is
  outside the current provider contract.
- Prompt rendering strips characters that Bash may expand from untrusted display
  values, including `$`, backticks, and backslashes.
- Protocol fields are data; neither endpoint evaluates them as shell source.
- There is no execute-command message.

MBX1 is a foundation protocol, not a complete provider schema. History RECORD
ingestion is specified separately as MBX2 (`docs/protocol-mbx2.md`) rather than
by extending MBX1. Completion and later interactive features that need typed
results, generation IDs, or cancellation remain a later MBX2 revision, not an
MBX1 change.
