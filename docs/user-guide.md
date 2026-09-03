# MBX in plain language

A guide for anyone who wants to use MBX without reading the engineering docs.
No project jargon here — just what it does, how to set it up, how to use it,
and what it can't do.

---

## What is MBX?

MBX is an add-on for the Bash shell on Linux. You keep using Bash exactly as
you always have — MBX just makes the terminal more helpful:

- **A smarter prompt** that shows where you are, what Git branch you're on,
  whether the last command failed, and a warning when you're on a server or
  in a production environment.
- **Suggestions from your own history** as you type (like "press → to finish
  this command you ran before").
- **A pick-list of completions** you can browse instead of guessing what Tab
  found.
- **Optional color** for the command you're typing.
- **A searchable record of your commands** (completely optional, stored only
  on your machine).

Two things never change, no matter what you turn on:

1. **Bash runs everything.** Your aliases, functions, options, key bindings,
   and history files keep working the way they always did.
2. **Nothing runs until you press Enter.** MBX can only put text in front of
   you. Suggestions, completions, and accepted words are just text in the
   input line. If you don't press Enter, nothing happens.

---

## What you need

- Linux, with **Bash version 5.0 or newer** (check with `bash --version`).
- A **real terminal window** (the features that watch your typing switch
  themselves off inside scripts or pipes — that's on purpose).
- The Rust toolchain, **version 1.85 or newer**, but only to build it once.
  There is no ready-made download yet.
- Git is optional. It only matters if you want the Git parts of the prompt
  and completions.

---

## Setting it up

### The easy way

From the top folder of the project:

```bash
bash scripts/install.bash --interactive
```

This builds the small companion program MBX uses, then asks you a numbered
set of questions (which features, which prompt style, which keys). Pick
**comfort** if you just want the best everyday setup — it enables the prompt,
history search, typing suggestions, and the completion pick-list.

The installer **never edits your `~/.bashrc` by itself**. To make MBX start
automatically in every new terminal, either choose that option in the menu,
or run:

```bash
bash scripts/install.bash --bashrc
```

This adds one clearly-marked block to `~/.bashrc`, and
`bash scripts/install.bash --uninstall-bashrc` removes it again.

### Other quick choices

```bash
bash scripts/install.bash --bashrc --profile highlight  # color instead of suggestions
bash scripts/install.bash --bashrc --profile prompt     # prompt improvements only
bash scripts/configure.bash                             # change answers later
```

You can re-run the interactive menu any time. It remembers your previous
answers, and the settings are saved in `~/.config/mbx/config.bash`.

### Then start a new shell (or re-read your setup)

MBX loads itself when Bash starts. If you're experimenting without editing
`~/.bashrc`, load it by hand:

```bash
source /path/to/ColorBash/bash/init.bash
```

### Check that it worked

Inside a shell where MBX is loaded:

```bash
mbx_doctor    # walks through every feature: what's on, what's off, and how to fix it
mbx_status    # one-line summary
```

If `mbx_doctor` says something is off, it also prints the exact setting that
fixes it. That's usually all the troubleshooting you need.

---

## What you get, feature by feature

### 1. A smarter prompt (always on)

The prompt is two lines. The first line tells you where you are; the second
(`>`) is where you type.

```text
~/projects/api  git:main ~2 ?1  exit 1
>
```

Reading the first line:

| Piece | Meaning |
| --- | --- |
| `~/projects/api` | Your current folder (`~` means your home folder) |
| `git:main` | You're in a Git repository, on branch `main` |
| `+2` | 2 staged changes |
| `~2` | 2 changed files (not staged) |
| `?1` | 1 file Git isn't tracking |
| `exit 1` | Your last command failed (the number is its error code) |
| `ssh:myserver` | You're working on a remote machine over SSH |
| `! PROD · host · user` | You asked for a loud production warning (`MBX_PRODUCTION_CONTEXT=1`) |
| `3s` | How long the last command took (only if you turn on timing, and only for 2 seconds or longer) |

Things that can't be shown (for example, the helper program is slow or
missing) are simply left out — the prompt always works.

### 2. Suggestions as you type — "ghost text" (off by default)

With this on, when you type something similar to a command you ran before,
the rest of that older command appears after your cursor as a hint.

Turn it on:

```bash
export MBX_HISTORY=1
export MBX_GHOST=1
```

What the keys do while a hint is showing:

| Key | What happens |
| --- | --- |
| → (right arrow) | Keep the whole hint |
| Alt-F or Ctrl-→ | Keep just the next word of the hint |
| Enter | Run **only what you typed** — the hint is thrown away |
| ←, Home, ↑, ↓ | Ignore the hint and do the normal thing |
| Ctrl-X then Ctrl-N / Ctrl-P | Look at other past commands that match what you typed |

Typing anything else clears the hint. Hints come only from **your own**
recorded history, and only when history recording is on.

### 3. Search your past commands — an easier Ctrl-R (needs history on)

Press `Ctrl-X` then `h` and MBX replaces what you typed with the closest
match from your history (what you typed counts as the start of the command;
an empty line gives you the newest command from the current folder). Press
the same keys again to flip through other matches. `Ctrl-X` then `l` brings
back what you had typed. Nothing runs until Enter.

Two useful preferences:

- `MBX_SEARCH_FAILED=1` — when you press the keys on an empty line, show
  commands that **failed** last time first (handy for re-running something
  that needs fixing).
- `MBX_SEARCH_REPO=1` — prefer commands you ran in this Git repository,
  wherever in it you were.

You can also search from the normal command line, without any keys:

```bash
mbx history search recent --limit 10     # newest commands
mbx history search prefix git            # commands starting with "git"
mbx history search cwd "$PWD"            # commands run in this folder
mbx history search fuzzy instal          # close matches ("install")
mbx history search failed                # ones that returned an error
mbx history search repo "$PWD"           # ones run anywhere in this repository
mbx history search branch main           # ones run on this branch
```

### 4. A pick-list for Tab completions (off by default)

Normal Tab completion is untouched. If you also turn on the overlay, MBX can
show a small menu of the matching choices below your prompt, with the best
one highlighted:

```bash
export MBX_COMP_OVERLAY=1
```

The menu works with completers MBX is "wrapped" around. The comfort setup
wraps Git for you. Then:

| Key | What happens |
| --- | --- |
| Tab | Normal Bash completion (unchanged) |
| Ctrl-X then Ctrl-O | Show or hide the menu |
| Ctrl-X then n / p | Move up and down the menu |
| Ctrl-X then Ctrl-A | Put the highlighted choice into your line |
| Ctrl-X then j | Close the menu |
| Ctrl-G | Unchanged (still Bash's normal cancel) |

The menu shows at most 8 choices, tidied so odd characters can't scramble
your terminal. If a completer isn't wrapped, the menu simply has nothing to
show and stays out of the way.

### 5. Color while you type (off by default)

```bash
export MBX_HIGHLIGHT=1
```

Your typing line stays plain. A colorized copy appears on one line below it:
shell keywords in bold blue, quoted text in green, `$variables` in yellow,
operators in magenta, numbers in cyan, comments in gray. When you press
Enter, the **plain** command runs — exactly the characters you typed, never
the colored copy.

Small print: only real shell keywords get the keyword color (so `true` and
`if` light up, but `echo` — which is a built-in program, not a keyword —
doesn't). Very long lines (over about 4,000 characters) and lines containing
raw control characters are left uncolored. While the completion menu is
open, coloring steps aside so the two don't fight over the screen.

### 6. Paste a snippet at a keypress (mostly for testing)

`Ctrl-X` then `Ctrl-Y` inserts a short piece of text (`MBX_EDITOR_INSERT_TOKEN`
sets what). It's aimed at developers exercising the shell, but it's there if
you want a text shortcut.

### 7. How long did that take? (off by default)

```bash
export MBX_ENABLE_DURATION_TIMING=1
```

Commands that take 2 seconds or longer show their duration on the next
prompt. It stays off by default because it doesn't touch Bash's existing
timing hooks unless you ask.

---

## Privacy: what gets remembered, and how to control it

Everything here is **off unless you turn it on** with `MBX_HISTORY=1`.

- When on, MBX keeps a small database (`SQLite`) of the commands Bash
  actually ran, with the folder, time, success/failure, and — in Git
  repositories — the repository and branch. **Your normal `~/.bash_history`
  file is never touched.**
- The database lives in `~/.local/share/mbx/` (or `$XDG_DATA_HOME/mbx/` if
  you set that), with file permissions so only your user can read it. It
  never leaves your machine, and it is never included in error messages or
  logs.
- Bash's own rules still decide what counts: commands starting with a space
  (with `ignorespace`), duplicates (with `ignoredups`), `HISTIGNORE`
  patterns, and history turned off — all behave exactly as before.
- Extra filter, just for this database:

  ```bash
  export MBX_HISTORY_EXCLUDE='aws *:export *: *token*'   # colon-separated patterns
  ```

- To wipe it:

  ```bash
  mbx history clear    # empty the database (keeps the file)
  mbx history delete   # remove the database files completely
  mbx history path     # show where it lives
  mbx history count    # how many commands are stored
  ```

One honest caveat: the database stores your command text **as plain text** on
your disk, protected by normal file permissions. If you type secrets into
commands, use the exclusion list for those patterns — same advice as for
`.bash_history` itself.

---

## The promises (in plain words)

1. Bash is still the boss — everything about how commands run is unchanged.
2. Text MBX offers you is only ever text. Enter is the only thing that runs
   a command.
3. If MBX's helper program crashes, hangs, or answers nonsense, you get a
   normal Bash prompt. A failed feature never breaks your shell.
4. Every extra feature is off until you explicitly set its variable to `1`.
5. MBX won't take over keys you already use. If a key combination is taken,
   MBX skips it (you can force it with the matching `MBX_*_OVERRIDE=1`
   setting, listed by `mbx_doctor`).
6. It never slows your typing down on purpose: if a lookup can't finish in
   time, the feature quietly skips that once rather than making you wait.

---

## Limits — what MBX does not do

- **Linux only, for now.** macOS support is written but untested; Windows is
  not supported. Bash 5.x only — not zsh, not fish.
- **Ghost hints and coloring can't be on together.** Pick one (the comfort
  profile picks hints; the `highlight` profile picks coloring).
- **No prebuilt downloads yet.** You build it from source once with Rust.
- **Suggestions only know what your history knows.** No cloud, no
  autocomplete of programs' internals, nothing network-based.
- **No type-to-filter Ctrl-R screen.** Today's search inserts a match into
  your line (feature 3); a live-filtering menu like fzf is not implemented.
- **The completion menu only covers completers that were wrapped** (Git by
  default in the comfort profile). Everything else keeps normal Tab.
- **The color copy is a display**, so copy-pasting from the preview row
  below your line isn't a thing — your real line is the plain one.
- **Heavy system load** can make a hint or a menu miss its moment; it will
  simply not appear that once instead of making you wait.
- Not implemented (on the project's wishlist): faded in-place ghost styling,
  live-filtering history menu, and prebuilt binaries.

---

## Turning things off or removing MBX

- One feature: set its variable to `0` (or unset it), e.g.
  `export MBX_GHOST=0`. Settings already in your shell beat the config file.
- Everything for this session: `mbx_configure` menu, or edit
  `~/.config/mbx/config.bash`.
- Stop MBX loading at startup:
  `bash scripts/install.bash --uninstall-bashrc` (removes the managed block)
  and delete the `source .../init.bash` line if you added one by hand.
- Delete your recorded history first if you want:
  `mbx history delete`.

---

## If something looks wrong

1. Run `mbx_doctor`. It names the problem and the fix for: Bash version,
   terminal capabilities, the helper program, each feature's key
   combinations (including conflicts with keys you already use), and the
   history database.
2. A feature not appearing? Check it's both enabled (`MBX_*`=1) **and**
   that you're in a real terminal window — these features switch off in
   scripts and pipes by design.
3. A key does the old thing instead of the new thing? Something else owned
   that key. `mbx_doctor` will say so, and the matching `MBX_*_OVERRIDE=1`
   is your call to make.
4. Want to see what a feature is doing? `mbx_status` is the quick summary;
   the [reference](reference.md) documents every switch in detail.

---

## Going deeper

- [`docs/reference.md`](reference.md) — every feature, every setting, with
  exact examples
- [`README.md`](../README.md) — quick start and project overview
- [`docs/roadmap.md`](roadmap.md) — what's done, what's next (this is where
  the engineering status lives)
