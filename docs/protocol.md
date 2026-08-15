# MBX1 protocol

## Goals

MBX1 is a small local request/response protocol used identically over coprocess
stdio and Unix streams. It is versioned, line-delimited, bounded to 64 KiB, and
simple enough for Bash to produce and validate with builtins.

## Framing

Each UTF-8 message occupies one line. Fields are separated by a tab. Literal `%`,
control characters, tabs, and line breaks inside a field are percent-encoded as
uppercase UTF-8 bytes. Decoders accept upper- or lowercase hex and reject malformed
escapes. Unescaped line terminators and NUL are invalid.

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

Unknown bits must be ignored within MBX1 so additive capability flags remain
forward-compatible.

## Security properties and limits

- Unix sockets are created with mode `0600` and never replace an existing path.
- Message and Git output sizes are bounded.
- Prompt rendering strips characters that Bash may expand from untrusted display
  values, including `$`, backticks, and backslashes.
- Protocol fields are data; neither endpoint evaluates them as shell source.
- There is no execute-command message.

MBX1 is a foundation protocol, not a complete provider schema. History and
completion will require structured payloads; an incompatible framing or trust
change must become MBX2 rather than silently changing MBX1.

