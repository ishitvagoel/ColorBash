# Architecture decision records

Index of the ADRs in this directory. Each file is authoritative for its own
decision; this table is only navigation. Statuses quote each record's own
`Status:` line — when a decision changes, amend the ADR itself (or supersede
it with a new one) and update the row here in the same change.

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](0001-bash-remains-execution-engine.md) | Bash remains the execution engine | Accepted |
| [0002](0002-rust-helper-architecture.md) | A small Rust helper supports Bash | Accepted for the foundation |
| [0003](0003-readline-vs-custom-editor.md) | Augment Readline before considering a custom editor | Accepted for MVP experiments |
| [0004](0004-ipc-transport.md) | Use a Bash coprocess for MVP IPC | Accepted for MVP |
| [0005](0005-history-storage.md) | History sidecar — privacy, capture, data, and protocol contract | Accepted (2026-08-15, `G1` decision) |
| [0006](0006-completion-integration.md) | Adapt existing Bash completion before enriching it | Proposed; experiment required (experiment delivered and gated `complete` — `docs/g4-gate-close-plan.md`) |
| [0007](0007-provider-model.md) | Providers return bounded data and reject implicit project execution | Accepted for the bounded Git prompt provider; broader model deferred |
| [0008](0008-history-prefix-index.md) | History many-match exact-prefix covering index (schema v2) | Accepted (2026-08-16, `HIST-007` `G2` evidence slice) |
| [0009](0009-explicit-history-search-bind-x.md) | Explicit history-search `bind -x` is Strategy A | Accepted (2026-08-16) |
| [0010](0010-opt-in-inline-ghost.md) | Opt-in inline ghost via stock self-insert wrapping | Accepted (2026-08-17) |
| [0011](0011-async-feature-ipc.md) | Async feature queries on MBX2 with generation IDs | Accepted (2026-08-20) |
| [0012](0012-macos-platform-matrix-deferral.md) | Defer macOS `HRD-001` pairwise PTY from Strategy A MVP | Accepted (2026-08-27) |
| [0013](0013-opt-in-continuous-decoration.md) | Opt-in continuous decoration via self-insert wrapping | Accepted (2026-08-27); install rules stand, decisions 2–3 superseded by [ADR 0015](0015-highlight-preview-row.md) |
| [0014](0014-highlight-over-coprocess.md) | Route opt-in highlighting through the coprocess | Accepted (2026-08-29); its `M-064` `color=0` deferral resolved by [ADR 0015](0015-highlight-preview-row.md) |
| [0015](0015-highlight-preview-row.md) | Preview-row syntax highlighting (supersedes in-buffer markers) | Accepted (2026-08-31) |

Consequential changes to protocol, privacy, persistence, editor ownership, or
provider execution still require a new ADR here or an explicit amendment of an
existing one (`AGENTS.md`, roadmap invariant 10).
