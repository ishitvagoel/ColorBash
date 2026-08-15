# Modern Bash UX — Codex Architecture & Agentic Development Brief

## 1. Purpose

Build a modern, colorful, highly interactive command-line experience that remains **strictly Bash-compatible**.

The product is **not a new shell language**. Bash remains the executing shell and source of truth for semantics.

The project should modernize the interactive Bash experience with:

- rich colors and icons
- adaptive prompt information
- live syntax highlighting
- history-based ghost suggestions
- richer completion menus
- searchable command history
- Git-aware context
- command descriptions
- intelligent error hints
- multiline editing improvements
- command/provider intelligence
- optional AI assistance
- fast, low-latency terminal rendering

The working product principle is:

> **It is Bash. It just feels like a modern developer tool.**

The project may use a native component—preferably Rust—for terminal rendering, indexing, ranking, parsing assistance, caching, and high-performance interactive UI, while preserving Bash itself as the command execution engine.

---

# 2. Non-Negotiable Product Constraints

## 2.1 Bash is the shell

Do not invent a replacement shell syntax.

The following must continue to work as Bash users expect:

- `.bashrc`
- `.bash_profile`
- aliases
- shell functions
- arrays
- parameter expansion
- command substitution
- process substitution
- pipelines
- redirects
- subshells
- traps
- job control
- environment variables
- exit codes
- globbing
- brace expansion
- Bash programmable completion
- Bash scripts
- `set -e`, `set -u`, `pipefail`, etc.
- terminal signals and foreground/background processes

The project enhances the **interactive editing and presentation layer**.

It must not silently reinterpret commands.

---

## 2.2 Commands remain ordinary Bash commands

If the interface helps the user construct:

```bash
git switch feature/auth
```

the actual command executed must still be:

```bash
git switch feature/auth
```

Do not hide shell operations behind a proprietary command graph.

---

## 2.3 Suggestions should not execute automatically

Features such as:

- autocomplete
- command palette
- AI command generation
- error recovery
- dangerous-command warnings

should normally **insert or propose text**.

The user retains control over execution.

---

## 2.4 Graceful degradation

The UX must work reasonably when:

- Nerd Fonts are absent
- colors are disabled
- terminal capabilities are limited
- the Rust helper is missing
- the terminal is accessed over SSH
- `TERM=dumb`
- Unicode rendering is unreliable

Icons must have text fallbacks.

Example:

```text
󰊢 project  main
```

fallback:

```text
git:project [main]
```

---

# 3. Product Philosophy

The project should optimize for four things:

1. **Compatibility**
2. **Discoverability**
3. **Speed**
4. **Progressive enhancement**

The shell should feel more capable without feeling magical.

Avoid UI clutter.

Information should appear when it becomes useful.

For example, a Git branch should appear inside a Git repository but not in `/tmp`.

A Python environment should appear when one is active.

A production context should be visually obvious.

---

# 4. User Experience Model

Think of the product as:

```text
┌───────────────────────────────────────────────┐
│                Interactive UX                 │
│                                               │
│ Prompt · Highlighting · Completion · History │
│ Suggestions · Help · Providers · Diagnostics │
├───────────────────────────────────────────────┤
│             Bash Integration Layer            │
│                                               │
│ Readline · Hooks · Completion · PS1 · DEBUG  │
├───────────────────────────────────────────────┤
│                   Bash                        │
│                                               │
│ Parsing · Execution · Jobs · Expansion       │
└───────────────────────────────────────────────┘
```

A native helper can exist beside the Bash integration:

```text
                ┌─────────────────────┐
                │     Rust Engine      │
                │                     │
                │ render              │
                │ history DB          │
                │ ranking             │
                │ cache               │
                │ providers           │
                │ fuzzy search        │
                │ terminal features   │
                └─────────┬───────────┘
                          │ IPC
                          │
┌─────────────────────────▼─────────────────────┐
│               Bash Integration                │
└─────────────────────────┬─────────────────────┘
                          │
                          ▼
                        Bash
```

---

# 5. Proposed Architecture

## 5.1 Recommended hybrid architecture

Use:

- **Bash** for shell integration and compatibility
- **Rust** for performance-sensitive interactive services
- optional embedded or local storage for history/context metadata
- simple structured IPC between Bash and Rust

Do not move Bash execution into Rust.

Rust should support Bash rather than replace it.

---

## 5.2 Why Rust

Rust is a strong candidate for:

- terminal control
- ANSI rendering
- fuzzy matching
- history indexing
- completion ranking
- asynchronous metadata collection
- Git status parsing
- caching
- provider plugins
- structured parsing helpers
- long-running helper daemon
- low-latency command-line UI
- reliable memory usage

Potential crates to evaluate later:

- `crossterm`
- `ratatui`
- `nix`
- `serde`
- `serde_json`
- `tokio`
- `rusqlite`
- `fuzzy-matcher`
- `git2`
- `unicode-width`
- `dirs`
- `clap`

Do not commit to any crate before validating whether it fits the latency and integration model.

---

# 6. Architectural Components

A possible repository structure:

```text
modern-bash/
├── README.md
├── docs/
│   ├── architecture.md
│   ├── ux-spec.md
│   ├── bash-compatibility.md
│   ├── protocol.md
│   └── adr/
├── bash/
│   ├── init.bash
│   ├── hooks.bash
│   ├── prompt.bash
│   ├── completion.bash
│   ├── keybindings.bash
│   └── fallback.bash
├── crates/
│   ├── cli/
│   ├── engine/
│   ├── renderer/
│   ├── history/
│   ├── completion/
│   ├── providers/
│   ├── terminal/
│   └── protocol/
├── providers/
│   ├── git/
│   ├── docker/
│   ├── python/
│   ├── node/
│   └── kubernetes/
├── themes/
├── tests/
│   ├── bash/
│   ├── integration/
│   ├── terminals/
│   └── snapshots/
└── scripts/
```

This is a starting proposal, not a rigid requirement.

Codex should refine it after experimentation.

---

# 7. Bash Integration Layer

The Bash layer should be deliberately small.

Responsibilities:

- initialize the product
- detect interactive Bash
- install hooks
- capture prompt lifecycle events
- capture command start/end metadata
- integrate with programmable completion
- expose keybindings
- communicate with native helper
- fall back safely when helper is unavailable
- maintain Bash semantics

Avoid putting expensive parsing, indexing, or Git operations directly inside prompt callbacks.

---

# 8. IPC Between Bash and Rust

Explore a small structured protocol.

Potential approaches:

### Option A — command invocation

```bash
mbx prompt --cwd "$PWD" --status "$?"
```

Simple, but process startup may become expensive.

### Option B — long-running helper daemon

```text
Bash
  │
  ├──── Unix socket ────► mbx-daemon
  │
  ◄──── response ───────
```

Advantages:

- warm caches
- faster ranking
- background metadata refresh
- centralized state

Risks:

- daemon lifecycle complexity
- stale state
- socket cleanup
- version mismatch

### Option C — coprocess

Use Bash `coproc` to keep a helper alive.

This may provide a useful middle ground.

Codex should prototype at least two approaches and benchmark them.

Target interactive latency should be low enough that the user cannot perceive UI lag.

---

# 9. Prompt UX

The prompt must be adaptive.

## Normal directory

```text
 ~/projects
❯
```

## Git repository

```text
󰊢 pocket-empires   main  +3 ~2
❯
```

## Python project

```text
 pocket-empires   main  Python 3.13
❯
```

## Previous failure

```text
󰊢 pocket-empires   main                  127
❯
```

## SSH

```text
󰒍 production-api   /srv/app
❯
```

## Production context

```text
󰀪 PROD · payments-api · root
❯
```

Production context should be intentionally visually prominent.

---

# 10. Prompt Information Hierarchy

Do not show everything all the time.

Priority order could be:

1. dangerous/production context
2. current working directory/project
3. Git branch/status
4. active environment
5. last command failure
6. execution duration if notable
7. host/SSH context
8. time only when useful

The prompt renderer should have a concept similar to:

```text
segment:
  priority
  visibility_condition
  compact_render
  full_render
  fallback_render
```

---

# 11. Color and Icon System

Use semantic roles rather than hard-coded colors.

Example semantic tokens:

```text
prompt.primary
prompt.secondary
status.success
status.warning
status.error
git.clean
git.modified
git.untracked
git.branch
path.directory
command.executable
command.argument
command.option
command.string
command.variable
command.operator
completion.selected
completion.description
danger.high
danger.critical
```

Themes should map semantic roles to terminal colors.

Support:

- 16-color terminals
- 256-color terminals
- true color
- no-color mode

Respect `NO_COLOR` where appropriate.

---

# 12. Live Syntax Highlighting

Typing should be visually parsed as the user edits the command.

Example:

```bash
docker compose up --detach
```

Semantic categories:

```text
docker      executable
compose     subcommand
up          subcommand
--detach    option
```

Other categories:

- executable
- subcommand
- option
- path
- glob
- string
- environment variable
- expansion
- operator
- pipe
- redirect
- command substitution
- comment
- unknown command
- syntactic error

Unknown commands may be visually flagged.

Avoid pretending to fully parse Bash unless Bash itself is the source of truth.

Any parser used for highlighting must tolerate incomplete input.

---

# 13. Dangerous Command Highlighting

The shell may visually identify potentially destructive commands.

Examples:

```bash
rm -rf ./build
```

Warning level.

```bash
rm -rf /
```

Critical level.

Potentially sensitive contexts:

- root shell
- production host
- destructive Git reset
- force push
- filesystem formatting
- recursive permission changes
- deletion outside working tree

Important rule:

**Do not change command semantics by default.**

Warnings are primarily visual.

Optional confirmation modes may be developed later behind explicit configuration.

---

# 14. Ghost Suggestions

History-based autosuggestions should appear as dim inline text.

Example:

```text
❯ git pu
       sh origin feature/authentication
```

Potential controls:

```text
Right Arrow      accept entire suggestion
Ctrl+Right       accept next word
Alt+Down         next suggestion
Alt+Up           previous suggestion
```

Ranking sources:

1. exact current-prefix history
2. history from current repository
3. history from current directory
4. recent global history
5. completion candidate
6. provider suggestion

Do not block keyboard input while suggestions are generated.

---

# 15. Completion UI

Traditional completion dumping should be replaced by an interactive menu when enhanced mode is active.

Example:

```text
❯ git switch fe

  Branches
  ┌──────────────────────────────┐
  │ 󰘬 feature/authentication     │
  │ 󰘬 feature/dashboard          │
  │ 󰘬 feature/payments           │
  └──────────────────────────────┘

↑↓ navigate   Tab accept   Esc close
```

The system should remain able to consume standard Bash programmable completion.

The UI layer should add:

- grouping
- sorting
- fuzzy matching
- descriptions
- icons
- recency
- provider metadata

---

# 16. Completion Adapter

Model:

```text
Bash completion ecosystem
          │
          ▼
completion adapter
          │
          ▼
normalization
          │
          ▼
ranking
          │
          ▼
semantic enrichment
          │
          ▼
interactive renderer
```

Normalized candidate schema could resemble:

```json
{
  "value": "feature/authentication",
  "display": "feature/authentication",
  "kind": "git_branch",
  "description": "Last used 14 minutes ago",
  "icon": "git_branch",
  "score": 0.94,
  "source": "git"
}
```

---

# 17. Semantic Completion

This should be one of the project's major differentiators.

Example:

```text
git switch <TAB>
```

could show:

```text
󰘬 main                    default branch
󰘬 dev                     3 commits ahead
󰘬 feature/payments        last used 14 min ago
󰘬 fix/mobile-layout       last used 4 days ago
```

Example:

```text
docker compose <TAB>
```

could show:

```text
up       Create and start containers
down     Stop and remove containers
build    Build services
logs     View service output
ps       List containers
```

The completion system should make the CLI more discoverable without changing command syntax.

---

# 18. History System

Do not limit history to a flat list of strings.

Store optional metadata:

```text
command
cwd
timestamp
duration
exit_code
repository
branch
hostname
session_id
shell_level
```

Potential storage:

```text
SQLite
```

Keep Bash history compatibility.

Do not destroy or replace `.bash_history`.

The enhanced database should be a sidecar.

---

# 19. Searchable History UI

`Ctrl+R` should become a full interactive search.

Example:

```text
Search history: docker postgres

  2h   ~/api       docker compose logs postgres
  1d   ~/api       docker compose restart postgres
  4d   ~/infra     docker exec postgres pg_dump ...
  8d   ~/sandbox   docker run postgres:18
```

Filters could eventually support:

```text
failed
successful
cwd
repo
today
host
branch
slow
```

Examples:

```bash
history --failed
history --cwd
history --today
history --slow
```

These may be exposed through a project-specific command rather than overriding Bash builtins.

Avoid incompatible shadowing unless explicitly designed.

---

# 20. Frecency-Based Navigation

Provide smarter directory navigation.

Example:

```text
Alt+C

> empire

󰉋 ~/projects/pocket-empires
󰉋 ~/projects/infinity-kingdom
```

Ranking can combine:

- frequency
- recency
- current repository
- directory hierarchy proximity

Avoid silently replacing normal `cd` behavior.

Potential enhanced command:

```bash
j pocket
```

or interactive keybinding.

---

# 21. Multiline Editing

Multiline Bash commands should look and behave like code.

Example:

```text
❯ docker run \
│   --name postgres \
│   -e POSTGRES_PASSWORD=test \
│   -p 5432:5432 \
╰─  postgres:18
```

Features:

- syntax highlighting
- indent awareness
- bracket matching
- quote matching
- multiline cursor motion
- safe paste handling
- terminal resize handling
- continuation prompt styling

---

# 22. Command Result Blocks

Experiment with visually separating command output.

Example:

```text
❯ npm test
──────────────────────────────────────
 PASS  tests/api.test.ts
 PASS  tests/user.test.ts

Tests: 34 passed
Time:  4.13s
──────────────────────────────────────
✓ 4.13s
```

Failure:

```text
──────────────────────────────────────
 exit 1 · 2.4s
```

This should be optional and subtle.

Do not interfere with programs that control the terminal directly.

---

# 23. Context-Aware Environments

Recognize useful developer context.

Initial candidates:

- Git
- Python
- Node.js
- Docker
- Kubernetes
- SSH
- virtualenv
- Conda
- AWS profile
- GCP project
- Azure subscription
- Terraform workspace

Examples:

```text
 pocket-empires · Python 3.13 · venv
```

```text
 dashboard · Node 22 · pnpm
```

```text
󱃾 prod-eu / payments
```

Metadata collection must be cached and asynchronous where possible.

Never run expensive commands on every keystroke.

---

# 24. Provider Architecture

Create provider interfaces.

Conceptual API:

```text
Provider
 ├─ detect(context)
 ├─ complete(input, context)
 ├─ describe(token, context)
 ├─ validate(input, context)
 ├─ suggest(input, context)
 ├─ prompt_segments(context)
 └─ diagnostics(command_result, context)
```

Providers should be independently testable.

Initial providers:

```text
git
filesystem
bash
python
node
docker
```

Later:

```text
kubectl
aws
gh
cargo
systemctl
terraform
```

---

# 25. Git Provider

Potential intelligence:

- repository root
- current branch
- detached HEAD
- dirty state
- staged changes
- unstaged changes
- untracked files
- branch candidates
- remote candidates
- tag candidates
- worktrees
- recent branches
- upstream relationship

Avoid calling `git status` synchronously for every keypress.

Use caching.

---

# 26. Docker Provider

Potential intelligence:

- running containers
- stopped containers
- images
- Compose services
- networks
- volumes
- common command descriptions

Example:

```text
docker logs <TAB>
```

could display running containers first.

---

# 27. Python Provider

Potential intelligence:

- virtual environment
- Python version
- installed CLI tools
- project type
- `pyproject.toml`
- `requirements.txt`
- `uv`
- Poetry
- pip
- common Python command descriptions

Avoid scanning large environments unnecessarily.

---

# 28. Node Provider

Potential intelligence:

- Node version
- package manager
- package scripts
- project name
- workspace
- executables from `node_modules/.bin`

Example:

```text
npm run <TAB>
```

could show scripts with descriptions.

---

# 29. Command Palette

A keyboard shortcut such as:

```text
Ctrl+Space
```

may open:

```text
What do you want to do?

> git bra

  󰊢 Switch Git branch
  󰊢 Create Git branch
  󰊢 Delete Git branch
   Show branches
```

Selecting an action should typically **insert Bash text** into the command line.

Do not execute automatically.

---

# 30. Contextual Help

A help key such as `F1` could show lightweight inline documentation.

Example:

```text
tar — archive utility

Common:
  tar -czf archive.tar.gz DIR
  tar -xzf archive.tar.gz
  tar -tf archive.tar.gz
```

For:

```bash
git reset --hard
```

the help system could highlight:

```text
--hard
Resets index and working tree.
Working-tree changes may be discarded.
```

Prefer documentation from trusted local sources when possible.

---

# 31. Error Intelligence

After a failed command, provide optional nonintrusive hints.

Example:

```text
❯ python manage.py runserver
python: can't open file 'manage.py': [Errno 2] No such file or directory

 Exit 2

Possible issue:
  manage.py does not exist in the current directory.

Nearby:
  ./src/manage.py

Suggested:
  python src/manage.py runserver
```

Another example:

```text
❯ git push

No upstream branch configured.

Suggested:
  git push --set-upstream origin feature/login
```

Suggestions must remain editable.

---

# 32. Optional AI Layer

AI must not be required for the core product.

Possible feature:

```text
Ctrl+G
```

opens:

```text
Describe command:
> find files larger than 500MB modified this week
```

Result:

```bash
find . -type f -size +500M -mtime -7
```

Actions:

```text
Enter     insert into line
Ctrl+E    explain
Esc       cancel
```

Never auto-execute AI-generated commands.

AI providers may include:

- disabled
- local model
- configurable remote API

Privacy behavior must be explicit.

---

# 33. Terminal Safety

The renderer must coexist with:

- `vim`
- `nano`
- `less`
- `top`
- `htop`
- `fzf`
- `tmux`
- `screen`
- SSH
- REPLs
- TUIs
- fullscreen programs

Do not corrupt terminal state.

Terminal state restoration must be tested after:

- Ctrl+C
- Ctrl+Z
- process crash
- shell exit
- Rust helper crash
- terminal resize
- SSH disconnect

---

# 34. Performance Requirements

Performance is part of the UX.

Initial targets:

- prompt render: effectively instantaneous
- normal keypress highlighting: no perceptible delay
- ghost suggestions: nonblocking
- completion popup: very low latency
- Git metadata: cached
- history search: interactive even with large histories

Set measurable benchmarks after the first prototype.

Avoid expensive subprocess invocation during every keystroke.

---

# 35. Compatibility Test Matrix

Test at minimum:

### Bash

- current stable Bash
- Bash 5.x variants
- interactive shell
- login shell
- nested Bash shell

### OS

- Linux
- WSL
- macOS where supported

### terminals

- Windows Terminal
- GNOME Terminal
- Konsole
- iTerm2
- Kitty
- Alacritty
- tmux
- SSH pseudo-terminal

### terminal capability

- true color
- 256 color
- 16 color
- no color
- Unicode
- no Nerd Font

---

# 36. Readline and Editing Strategy

One major architectural investigation should determine whether the product should:

### Strategy A

Extend Bash + Readline behavior through Bash hooks/keybindings.

### Strategy B

Use a deeper interactive editing integration similar in spirit to advanced Bash line editors.

### Strategy C

Implement a custom editor frontend that still delegates final command execution to Bash.

Strategy C offers maximal UX control but carries the highest compatibility risk.

Codex should write an ADR comparing these approaches before committing.

---

# 37. Compatibility Rule

Whenever there is a conflict between:

```text
cool UX
```

and:

```text
correct Bash behavior
```

choose correct Bash behavior.

---

# 38. Configuration

Proposed config location:

```text
~/.config/mbx/config.toml
```

Example concept:

```toml
[ui]
icons = "auto"
color = "auto"
animations = false

[prompt]
git = true
duration_threshold_ms = 2000

[history]
enhanced = true

[suggestions]
history = true

[ai]
enabled = false
```

Do not overdesign configuration before the MVP.

---

# 39. Themes

Themes should control semantic presentation without changing behavior.

Example:

```text
themes/
├── default.toml
├── minimal.toml
├── high-contrast.toml
└── no-icons.toml
```

Theme inheritance can come later.

---

# 40. Project Commands

The native binary could temporarily be called:

```text
mbx
```

This is a placeholder name.

Potential developer-facing commands:

```bash
mbx doctor
mbx benchmark
mbx debug prompt
mbx debug completion
mbx debug provider git
mbx config
mbx theme
```

Avoid exposing too many commands to end users during MVP.

---

# 41. `mbx doctor`

A diagnostics command would be extremely useful.

It could inspect:

- Bash version
- interactive shell state
- terminal
- color capability
- Unicode capability
- Nerd Font likelihood
- helper connectivity
- socket state
- configuration
- completion integration
- keybinding collisions

Output should provide actionable fixes.

---

# 42. Observability for Development

Include debug modes early.

Examples:

```bash
MBX_LOG=trace
MBX_DISABLE_GIT=1
MBX_DISABLE_RENDERER=1
```

Potential tracing events:

```text
keypress
parse duration
history lookup duration
provider lookup duration
render duration
IPC latency
prompt callback duration
```

Never log sensitive command data by default.

---

# 43. Privacy

Enhanced history can contain sensitive commands.

Requirements:

- local storage by default
- no telemetry of command text
- clearly documented storage path
- ability to disable enhanced history
- configurable exclusions
- possible commands prefixed with space may respect Bash `HISTCONTROL` semantics
- avoid storing secrets where detectable
- AI integration must clearly state what command text leaves the machine

---

# 44. Security

Do not let provider metadata execute arbitrary repository code.

Examples:

- do not automatically source unknown files
- do not execute project-local scripts merely to discover metadata
- treat repository contents as untrusted
- sanitize terminal escape sequences from external data
- sanitize branch names and filenames before rendering
- guard against OSC/ANSI injection
- validate IPC messages
- secure Unix socket permissions

This is a terminal application; output injection is a real concern.

---

# 45. MVP — Version 0.1

Build only:

1. adaptive prompt
2. semantic colors/icons
3. live syntax highlighting
4. history-based ghost suggestions
5. interactive completion menu
6. searchable enhanced history
7. Git context

The MVP should already feel useful without AI.

---

# 46. Version 0.2

Add:

- command descriptions
- richer completion metadata
- directory frecency
- contextual error hints
- multiline editing refinements
- Python provider
- Node provider
- Docker provider

---

# 47. Version 0.3

Add:

- provider SDK
- command palette
- Kubernetes provider
- cloud providers
- theme/plugin ecosystem
- optional AI assistance
- richer diagnostics
- possible command output blocks

---

# 48. UX Prototype Scenarios

Before implementing everything, create terminal UX prototypes for these flows.

## Scenario A — Git checkout/switch

User types:

```text
git sw
```

Observe:

- highlighting
- ghost suggestion
- completion activation
- branch metadata
- selection
- insertion

---

## Scenario B — Docker Compose

User types:

```text
docker compose
```

Observe:

- subcommand help
- menu grouping
- provider metadata
- completion descriptions

---

## Scenario C — Failed command

Run:

```text
python manage.py runserver
```

when `manage.py` does not exist.

Observe:

- exit code
- prompt state
- diagnostic hint
- nearby file suggestion

---

## Scenario D — History

Press:

```text
Ctrl+R
```

Search:

```text
docker postgres
```

Observe:

- latency
- metadata
- cwd ranking
- selection behavior

---

## Scenario E — Multiline command

Type:

```bash
docker run \
  --name postgres \
  -p 5432:5432 \
  postgres
```

Observe:

- indentation
- continuation prompt
- cursor movement
- highlighting

---

## Scenario F — Production context

SSH into a host identified as production.

Observe:

- prompt visibility
- danger treatment
- fallback behavior

---

# 49. Agentic Development Principles for Codex

Codex should work incrementally.

For every major feature:

1. inspect existing architecture
2. update design notes if necessary
3. implement the smallest coherent vertical slice
4. add tests
5. run tests
6. benchmark if latency-sensitive
7. document behavior
8. record architectural decisions
9. only then expand scope

Avoid broad rewrites unless an ADR justifies them.

---

# 50. Architecture Decision Records

Create ADRs under:

```text
docs/adr/
```

Initial ADRs should include:

```text
0001-bash-remains-execution-engine.md
0002-rust-helper-architecture.md
0003-readline-vs-custom-editor.md
0004-ipc-transport.md
0005-history-storage.md
0006-completion-integration.md
0007-provider-model.md
```

Each ADR should contain:

```text
Context
Decision
Alternatives
Consequences
Risks
Validation plan
```

---

# 51. First Codex Investigation

Before writing a large amount of production code, Codex should answer:

### Bash lifecycle

- What reliable hooks exist before command execution?
- What hooks exist after command execution?
- How should `$?` be captured safely?
- How can timing be measured without altering behavior?
- What can be implemented through `PROMPT_COMMAND`?
- What limitations exist?

### Readline

- Which keybindings can be safely intercepted?
- How can current line contents be accessed?
- Can completion results be intercepted/enriched?
- How should multiline editing be handled?

### Completion

- How can existing Bash completion definitions be reused?
- How can `COMP_WORDS`, `COMP_CWORD`, `COMP_LINE`, and `COMPREPLY` be bridged into richer UI?
- Which completion edge cases are difficult?

### Rendering

- How can the current line be redrawn without flicker?
- How should terminal width changes be handled?
- How should double-width Unicode characters be handled?
- How should prompt escape sequences be accounted for?

### Performance

- What is the startup cost of a Rust process?
- Does a coprocess outperform per-invocation helpers sufficiently?
- Would a Unix socket daemon be justified?

Document findings before locking architecture.

---

# 52. Prototype Order

Recommended experimental order:

## Prototype 1

Bash initialization + Rust CLI handshake.

Prove:

```text
Bash ↔ Rust
```

without affecting command execution.

## Prototype 2

Adaptive prompt.

Prove:

- Git branch
- exit status
- execution duration
- icon fallback

## Prototype 3

History sidecar.

Prove:

- command capture
- cwd
- timestamp
- duration
- exit status
- fast search

## Prototype 4

Ghost suggestion.

Prove:

- current buffer capture
- history lookup
- inline rendering
- suggestion acceptance

## Prototype 5

Completion menu.

Prove:

- Bash completion reuse
- candidate normalization
- popup navigation
- insertion back into line

## Prototype 6

Syntax highlighting.

Prove:

- incomplete Bash input handling
- redraw performance
- quote/operator/path highlighting

Only then expand provider intelligence.

---

# 53. Testing Strategy

## Unit tests

Rust:

- ranking
- history queries
- theme resolution
- provider output
- ANSI sanitization
- width calculations
- protocol serialization

Bash:

- hook installation
- fallback behavior
- environment preservation
- prompt status capture

---

## Integration tests

Use pseudo-terminals.

Test flows such as:

```text
type command
press Tab
press Ctrl+R
press Ctrl+C
resize terminal
run foreground process
run background process
exit shell
```

Snapshot terminal output where practical.

---

## Compatibility tests

Verify ordinary Bash behavior before and after enabling the product.

Create a Bash corpus containing:

```bash
echo hello
echo "$HOME"
false || echo fallback
printf '%s\n' a b c | grep b
for x in a b; do echo "$x"; done
foo() { echo foo; }
alias ll='ls -l'
sleep 1 &
jobs
(cd /tmp && pwd)
cat <<< "hello"
echo <(printf test)
```

The UX layer must not alter their semantics.

---

# 54. Definition of Done for MVP

The MVP is successful when:

- a user installs it into Bash
- existing `.bashrc` configuration still works
- Bash remains the process executing commands
- prompt rendering feels instantaneous
- Git context appears correctly
- syntax highlighting works for common Bash input
- history ghost suggestions work
- enhanced Ctrl+R works
- completion menu works for standard completion
- helper failure does not break the shell
- icons gracefully degrade
- Ctrl+C does not corrupt the prompt
- fullscreen terminal applications behave normally
- SSH and tmux remain usable
- no command is executed automatically by suggestions
- compatibility tests pass

---

# 55. Explicit Non-Goals for MVP

Do not initially build:

- a new shell language
- a shell script interpreter
- remote cloud history sync
- AI-first workflow
- plugin marketplace
- graphical desktop application
- terminal emulator
- replacement for tmux
- replacement for Bash parser
- auto-fixing commands without user consent

---

# 56. Product Differentiation

The product should not be merely:

```text
Starship + bash-completion + fzf
```

The differentiation is the integration of:

```text
Bash compatibility
       +
semantic editing
       +
context-aware completion
       +
rich history
       +
provider intelligence
       +
modern terminal UX
```

The strongest potential differentiator is **semantic completion**.

Completion entries should become structured objects with:

- type
- description
- icon
- contextual relevance
- recency
- ranking
- provider metadata

while still inserting ordinary Bash syntax.

---

# 57. UX Rules

1. Do not surprise the user.
2. Do not hide Bash.
3. Do not execute suggestions automatically.
4. Do not block typing.
5. Do not clutter the prompt.
6. Do not require Nerd Fonts.
7. Do not require AI.
8. Do not perform expensive work on each keypress.
9. Do not break terminal-native applications.
10. Do not sacrifice Bash compatibility for aesthetics.

---

# 58. Initial UI Design Language

Aim for:

- compact
- information-dense without being noisy
- crisp separators
- semantic iconography
- strong failure/warning states
- restrained animation
- consistent key hints
- minimal borders
- terminal-native feel

Avoid making it look like a desktop GUI squeezed into a terminal.

---

# 59. Candidate Naming — Temporary Only

Until branding is decided, use an internal codename such as:

```text
mbx
```

Possible interpretations:

```text
Modern Bash Experience
Modern Bash UX
Bash Experience Layer
```

Do not spend engineering time on naming yet.

---

# 60. Codex Execution Checklist

## Phase 0 — Research / Architecture

- [ ] Inspect repository state.
- [ ] Create `docs/architecture.md`.
- [ ] Create `docs/ux-spec.md`.
- [ ] Create `docs/bash-compatibility.md`.
- [ ] Create initial ADR directory.
- [ ] Investigate Bash prompt lifecycle.
- [ ] Investigate Readline integration.
- [ ] Investigate Bash programmable completion reuse.
- [ ] Investigate multiline behavior.
- [ ] Prototype Rust startup latency.
- [ ] Prototype Bash ↔ Rust communication.
- [ ] Compare per-command helper vs coprocess vs daemon.
- [ ] Document findings.
- [ ] Select MVP architecture through ADR.

## Phase 1 — Bootstrap

- [ ] Create Rust workspace.
- [ ] Create Bash initialization script.
- [ ] Add installer/dev setup.
- [ ] Detect interactive Bash safely.
- [ ] Add graceful helper fallback.
- [ ] Add debug logging.
- [ ] Add CI.
- [ ] Add basic compatibility tests.

## Phase 2 — Prompt

- [ ] Build terminal capability detection.
- [ ] Build semantic theme model.
- [ ] Implement path segment.
- [ ] Implement Git segment.
- [ ] Implement exit-status segment.
- [ ] Implement command-duration segment.
- [ ] Implement SSH context.
- [ ] Implement icon fallback.
- [ ] Test prompt width.
- [ ] Test terminal resize.
- [ ] Benchmark prompt latency.

## Phase 3 — History

- [ ] Design sidecar history schema.
- [ ] Respect Bash history behavior.
- [ ] Capture command metadata.
- [ ] Store cwd.
- [ ] Store timestamp.
- [ ] Store exit status.
- [ ] Store duration.
- [ ] Store repo context.
- [ ] Add history query API.
- [ ] Add fuzzy ranking.
- [ ] Add privacy exclusions.
- [ ] Benchmark large history datasets.

## Phase 4 — Ghost Suggestions

- [ ] Access current editing buffer safely.
- [ ] Query ranked history.
- [ ] Render ghost text.
- [ ] Accept full suggestion.
- [ ] Accept word.
- [ ] Cycle suggestions.
- [ ] Ensure zero blocking during lookup.
- [ ] Test resizing and multiline interaction.

## Phase 5 — Completion

- [ ] Capture Bash completion state.
- [ ] Reuse existing completion functions.
- [ ] Normalize candidates.
- [ ] Add candidate kinds.
- [ ] Add fuzzy ranking.
- [ ] Build completion popup.
- [ ] Add keyboard navigation.
- [ ] Insert selected candidate correctly.
- [ ] Preserve quoting rules.
- [ ] Preserve whitespace rules.
- [ ] Test file completion.
- [ ] Test Git completion.
- [ ] Test nested subcommands.

## Phase 6 — Syntax Highlighting

- [ ] Define token taxonomy.
- [ ] Support incomplete input.
- [ ] Highlight executables.
- [ ] Highlight options.
- [ ] Highlight strings.
- [ ] Highlight variables.
- [ ] Highlight redirects.
- [ ] Highlight pipes/operators.
- [ ] Highlight paths.
- [ ] Identify unknown executables.
- [ ] Add dangerous-command visual classification.
- [ ] Benchmark per-keypress latency.

## Phase 7 — Git Provider

- [ ] Detect repository.
- [ ] Cache repository metadata.
- [ ] Expose branches.
- [ ] Expose remotes.
- [ ] Expose tags.
- [ ] Expose dirty state.
- [ ] Expose upstream state.
- [ ] Add branch descriptions.
- [ ] Add provider-specific completion ranking.

## Phase 8 — Enhanced Ctrl+R

- [ ] Build interactive history search.
- [ ] Show timestamp/age.
- [ ] Show cwd.
- [ ] Show exit status where useful.
- [ ] Support repo filtering.
- [ ] Support failed-command filtering.
- [ ] Insert selected command without executing.
- [ ] Test with 100k+ history entries.

## Phase 9 — Hardening

- [ ] Test tmux.
- [ ] Test SSH.
- [ ] Test WSL.
- [ ] Test fullscreen programs.
- [ ] Test Ctrl+C.
- [ ] Test Ctrl+Z.
- [ ] Test shell nesting.
- [ ] Test helper crash.
- [ ] Test Unicode width.
- [ ] Test terminals without Nerd Fonts.
- [ ] Test `NO_COLOR`.
- [ ] Test terminal escape injection.
- [ ] Run Bash compatibility corpus.
- [ ] Run latency benchmarks.

---

# 61. Codex Working Rules

When implementing this project:

- Do not make large speculative rewrites.
- Do not change Bash semantics to simplify implementation.
- Do not execute generated commands automatically.
- Do not introduce AI dependencies into core architecture.
- Do not add heavyweight frameworks unless justified.
- Prefer vertical slices that can be manually tested.
- Add tests with each subsystem.
- Benchmark latency-sensitive changes.
- Keep the Bash layer understandable.
- Keep protocol boundaries explicit.
- Record meaningful architectural decisions.
- Preserve fallback behavior throughout development.

When uncertain, prioritize:

```text
correctness > compatibility > latency > UX polish > feature breadth
```

except that severe latency regressions should be considered correctness issues for interactive features.

---

# 62. First Assignment for Codex

Start with **architecture discovery**, not the full implementation.

Perform the following:

1. Create the initial documentation structure.
2. Investigate Bash lifecycle hooks and Readline constraints.
3. Build a minimal Bash initialization script.
4. Build a minimal Rust binary.
5. Demonstrate Bash → Rust communication without changing command behavior.
6. Benchmark:
   - launching the Rust helper per prompt
   - keeping it alive as a Bash coprocess
   - communicating over a Unix socket if practical
7. Produce an ADR recommending the IPC model.
8. Produce an ADR comparing:
   - Readline augmentation
   - advanced Bash line-editor integration
   - custom frontend editor with Bash execution backend
9. Build a tiny adaptive prompt prototype.
10. Add a compatibility smoke-test suite.
11. Stop and reassess the architecture before implementing autocomplete or syntax highlighting.

The goal of the first assignment is to validate the foundation.

Do not prematurely build the entire UI.

---

# 63. Expected Initial Deliverables

Codex should leave the repository with something resembling:

```text
docs/
  architecture.md
  ux-spec.md
  bash-compatibility.md
  adr/
    0001-bash-remains-execution-engine.md
    0002-rust-helper-architecture.md
    0003-readline-vs-custom-editor.md
    0004-ipc-transport.md

bash/
  init.bash
  hooks.bash
  fallback.bash

crates/
  cli/
  protocol/

tests/
  bash/
  integration/
```

plus:

- working development setup
- basic Rust binary
- basic Bash loader
- simple prompt prototype
- benchmark results
- compatibility smoke tests
- documented next-step recommendation

---

# 64. Long-Term Vision

The final experience should feel like a modern code editor compressed into a terminal interaction model, while still respecting Unix and Bash.

A user should be able to install the project and immediately gain:

```text
better visibility
better completion
better history
better discoverability
better safety cues
better navigation
better context
```

without having to relearn shell syntax.

The long-term promise remains:

> **Bash underneath. A modern developer experience on top.**
