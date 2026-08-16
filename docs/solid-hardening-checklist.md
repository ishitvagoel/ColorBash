# SOLID foundation hardening checklist

Status: complete (2026-08-15 UTC)

This checklist is the execution record for the bounded SOLID findings already
approved in `docs/roadmap.md`. The roadmap remains the status authority. This
work package is limited to `FND-002`, `FND-003`, `BST-006`, `BST-007`, `PRM-003`,
`PRM-007`, `PRM-008`, and the shared `GIT-002` slice.

Do not start PTY/editor work, history, completion, suggestions, highlighting,
provider expansion, or any other roadmap item as part of this checklist.

## Transport and service contracts

- [x] Make transport own response correlation IDs and framing postconditions;
      application handlers return response content only.
- [x] Test a direct `RequestHandler` substitute for exact decoded input, one call,
      sentinel output, malformed-input bypass, and loop continuation.
- [x] Apply the 65,536-byte payload limit independently of EOF, LF, or CRLF.
- [x] Cover `MAX-1`, `MAX`, and `MAX+1` for EOF, LF, and CRLF.
- [x] Cap Bash response acquisition at the payload limit plus the bounded CRLF
      allowance before allocating the complete peer response; retain the wait
      deadline and fallback behavior.
- [x] Prove Unix-socket collision refusal, `0600` mode, normal cleanup, and
      correlation validation.

## Prompt adapter contract

- [x] Require the same explicit `(status, duration, cwd, flags)` context for
      coprocess, per-call, and fallback adapters.
- [x] Preserve the raw additive flag value through every adapter, including
      unknown bits in per-call mode.
- [x] Replace the full C0/DEL control range plus Bash expansion characters in the
      fallback renderer, using the same hostile corpus as native rendering.
- [x] Cover production precedence and an SSH-only fallback case.
- [x] Prove color-enabled per-call behavior under real command-substitution
      topology.
- [x] Keep `bash/prompt.bash` as the only prompt-path `PS1` writer.

## Bounded prompt and Git provider

- [x] Give the complete Bash render attempt one deadline budget across coprocess,
      cleanup, per-call, and final fallback.
- [x] Ensure the final Bash fallback performs no external process lookup.
- [x] Acquire Git stdout with a hard `MAX+1` cap and process deadline; attempt
      direct-child kill/reap on timeout and type cleanup failures. Distinguish
      timeout, oversize, spawn failure, and absence. A descendant holding
      inherited stdout cannot extend prompt return, but portable process-tree
      termination is not claimed.
- [x] Keep Git parsing pure and test command construction, disabled filesystem
      monitoring, hostile output, timeout, oversize, and failure.
- [x] Add a warm TTL cache around the repository-status port with deterministic
      hit, expiry, and invalidation tests.
- [x] Preserve silent prompt degradation while retaining command-text-free
      diagnostics for provider failure.

## Port and regression evidence

- [x] Map every `PromptContext` field, including unknown flags and nonzero
      status/duration, through `ProtocolService`.
- [x] Prove `PING` never invokes prompt rendering.
- [x] Prove provider `Err` omits only the repository segment and that the disabled
      flag never invokes the provider.
- [x] Compile crate-internal substitutes that construct both `Theme` and
      `ProviderError` from a sibling module.
- [x] Run focused tests, `bash tests/run.bash`, release-mode prompt/IPC benchmarks,
      `git diff --check`, and the mistake-log schema check.

## Explicitly deferred

- [x] Leave semantic prompt composition versus typed PS1 encoding at `PRM-009`
      discovery until `PRM-002` or a second renderer proves the change axis.
- [x] Leave direct-CLI redirected-color policy at `PRM-002`/`M-009`.
- [x] Leave PTY and all later product phases untouched.

## Completion evidence

- `bash tests/run.bash`: 50 CLI tests, 5 protocol tests, Rust doc tests,
  formatting/lint/build checks, Bash module contracts, protocol integration, and
  compatibility smoke all passed on 2026-08-15.
- GitHub Actions workflow `CI` on `origin/main` at commit
  `5c077ce1e9a51a89528bf97f4a4aa9b038148bd2` (push, conclusion `success`,
  2026-08-16T07:04:09Z):
  https://github.com/ishitvagoel/ColorBash/actions/runs/31932933113. The workflow
  runs `bash tests/run.bash` per `.github/workflows/ci.yml`.
- Release warm-Git prompt, 1,000 iterations: p50 718 us, p95 974 us, p99
  1,383 us.
- Release IPC, 1,000 iterations: process-per-call mean 1.068 ms, guarded Bash
  coprocess mean 0.500 ms, and persistent Unix-socket mean 0.048 ms.
- Detailed reproducibility and scope are recorded in
  `docs/benchmarks/2026-08-15-solid-hardening.md`.

Completing this bounded checklist completes `FND-001` and `BST-005` CI linkage
via the green GitHub Actions run above. It does not complete `G0`. The real
PTY/platform matrix and representative `PRM-004` percentiles remain separate
gate conditions.
