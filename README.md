# imp

A small terminal agent for DeepSeek, OpenAI, Anthropic, Gemini, and local
models. One binary, a chat interface, and a deliberately short list of tools.

```
                 binary   cold start   idle RSS
M1 MacBook Air   950 KB         4 ms     6.8 MB
Pi Zero 2 W      1.1 MB        45 ms     8.3 MB
```

libcurl is the only runtime dependency. Release profile `lto = "fat"`,
`opt-level = "z"`, `panic = "abort"`, stripped. Cold start is the median of
30 runs of `imp --version`; RSS is the full-screen interface idle for two
seconds. `scripts/measure.sh [host]` takes these figures.

## Documentation

- [Getting started](docs/getting-started.md): install, the first run, a first
  task, agents, sessions, and running on a Pi.
- [The agent loop](docs/agent-loop.md): what a turn does, from the system
  prompt through tool calls, permissions, retries, and context limits.
- This README is the reference: every command, key, setting, and limit.
- The behaviour imp must keep is specified in [`openspec/specs/`](openspec/specs/).

## Install

```sh
cargo build --release
install -m 755 target/release/imp ~/.local/bin/imp
imp init
export DEEPSEEK_API_KEY=...
```

`imp init` creates `~/.config/imp/` and `~/.local/state/imp/`, writes a
commented config and one example skill, and overwrites nothing. The agents
`build`, `plan` and `review` are built in; a file of the same name replaces one.

Needs libcurl at runtime: present on macOS and on any Linux with curl installed.

`scripts/deploy.sh host…` builds on this machine with `cargo-zigbuild` and
installs over SSH: it asks each device for its architecture and glibc, links
against the device's own libcurl, copies the binary to `~/.local/bin/imp`, and
keeps the previous one as `imp.prev`. A Pi Zero 2 W takes 18 s this way.
`--native` copies the source and builds on the device instead: 3½ minutes on
the same Pi. Cross builds need `zig`, `cargo install cargo-zigbuild`, and
`rustup target add aarch64-unknown-linux-gnu`.

### Keeping it running

imp has no daemon. Run it under tmux or dtach so a closed terminal or a
dropped SSH connection leaves every session working:

```sh
tmux new -As imp imp            # attach, or start if absent
dtach -A /tmp/imp.sock imp      # the same, smaller
ssh -t user@host tmux new -As imp imp   # run elsewhere, draw here
```

## Use

```sh
imp                                  # full-screen chat
imp "why is the build failing?"      # one request, then exit
imp find the biggest file             # unquoted words join into one prompt
imp -c                               # continue the last session
imp -r 3k9a0aa                       # resume a specific session
imp --cwd ~/Projects/imp             # run the session in another directory
printf 'ls\n/quit\n' | imp           # scripted line mode
```

Both front-ends share the session, tools, and history. Line mode is chosen when
stdin or stdout is not a terminal, and prints no escape sequences.

### Keys

```
Enter send · Alt+Enter newline · Esc cancel · Ctrl+C clear or quit · Ctrl+L clear
Up/Down history · Tab complete · PgUp/PgDn or the mouse wheel scroll
Picker: type to filter · Up/Down move · PgUp/PgDn page · Enter choose · Tab/Right open folder · Left up · Esc cancel
Ctrl+E expand the last tool call · Ctrl+R retry · Ctrl+T thinking · Ctrl+V paste the copy · Ctrl+G release the mouse · Ctrl+D leaves line mode
Ctrl+O next waiting · Ctrl+N next session · Ctrl+P palette · ? keys, from an empty composer
```

### Several sessions

`/new` and resuming from `/sessions` open another live session beside the
current one, up to four; the oldest idle one closes to make room and stays in
`/sessions`, and its background jobs stop, as every session's do when imp exits. Ctrl+N moves to the next one.
The title bar shows which one is in view, such as `2/3 fix the parser`, and
the status line how many others are `waiting`. Set `title_gradient = on` for a
title bar that shades toward the accent on a truecolor terminal; it is off by default. There is no close
command: `/new` moves on, and the pool makes room by itself.

In `/sessions`, a spinner marks a live session that is still working. When the
filter matches no title, Enter searches what was said in every session and
lists the hits with a snippet.

Ctrl+O or `/next` goes to the session that has waited longest for you: an
approval, a question, or a reply that finished out of view. Pressing it again
counts the reply as read and moves on; an approval is never answered that way,
it goes to the back of the queue. With `auto_next=on`, the default, sending a
message or answering a prompt moves you to the next waiting session, and a
line such as `→ 2/3 fix the parser · replied` says where you are. A prompt waits until you answer it, in view or out.
Approvals (except `a`), modes, and buffers belong to each session, and all sessions share
one 16 MB output buffer budget.

### Commands

```
/help      the commands          /keys      the keys
/agent     switch profile
/new       another live session  /reload    re-read config, instructions, skills
/sessions  pick a session        /compact   compact now
/think     thinking toggle       /mode      normal, strict, or yolo
/verbosity low, medium, high     /cwd       show the session directory
/cd        browse or change cwd  /image     browse or attach an image
/fork      copy the session      /theme     set the colour theme
/shell     open a shell          /model     set or pick the session model
/diff      what imp changed; `/diff git [args]` for git diff
/provider  manage providers      /queue     messages waiting to send
/rename    rename this session   /quit      leave
/next      next session waiting on you
/settings  the settings form: change ten settings, saved to the config
/retry     send the last message again, dropping its reply
/effort    low, medium, high, or default
```

Tab on an empty draft switches agent and Shift+Tab steps the mode through
normal, strict, and yolo. Ctrl+C clears a draft or stops a turn; idle, a
second Ctrl+C within two seconds quits. In the draft, Option or Alt with ←/→ moves by
word, Option+Backspace or Ctrl+W deletes one, Shift with the arrows selects,
and Ctrl+A, Ctrl+U, and Ctrl+K act on the row.

`/agent`, `/model`, `/theme`, `/verbosity`, `/provider`, and `/sessions` are
value commands. Bare, each opens a centred picker on the current entry, marked
`current`; a typed value such as `/theme mono` or `/sessions 51280befc` applies
directly. Line mode prints the same values as aligned rows. The filter has the
cursor: type to narrow, move with Up/Down or PgUp/PgDn, choose with Enter, and
cancel with Esc. A space hands the line back to the composer, so
`/provider add work` types straight through.

Tab in the command list completes the highlighted command: a value command
opens its picker, `/queue` and `/rename` get a space for their text, and any
other command waits for Enter.

Command rows show the command on the left and its description on the right.
Bare `/cd` browses directories from the session root. Enter makes the
highlighted folder the session root; Tab or Right opens it; Left goes up,
including above the root. The list starts with `./`, the folder on screen, so
Enter there chooses where you are, then `../`. The picker title names the
directory being browsed. Bare `/image` browses directories and PNG, JPEG, GIF,
and WebP files: Enter, Tab, or Right opens a folder, and Enter attaches the
highlighted image.

`/image <path>` or pasting an image path attaches PNG, JPEG, GIF, or WebP up to
10 MB to the next message.

`/shell` hands the terminal to `$SHELL` and redraws the chat when it exits.
`/model <name>` sets the session model; bare `/model` lists the provider's
models, fetched live with a built-in catalog as the immediate and fallback
list, plus the current model, the provider default, and recent choices.
Nothing is written to the config.

Bare `/provider` picks from built-in and custom profiles. `add <name>`, `edit <name>`,
`delete <name>`, and `use <name>` manage custom profiles in a managed
`# >>> providers` block in the config file; comments and other keys survive,
and the active profile cannot be deleted.

While a turn runs, Enter steers it or queues the message; Ctrl+Q switches
between the two, and the prompt glyph says which: `>` steers, a muted `»`
queues. Queued messages
wait above the composer and go out as turns end. ↑ on an empty draft opens
them for editing, which holds the turn; saving one empty removes it.
`/queue <text>` adds one, bare `/queue` lists them, `pop [n]` loads one into
the composer, `delete n` removes one, `clear` empties the queue, and
`pop all` loads every entry. The queue belongs to the session.
`/rename <title>` retitles the current session.

A line starting with `!` runs as a shell command, unchecked by the model, with
the same limits as the `bash` tool.

## Tools

`bash` · `read` · `edit` · `write` · `fetch` · `search` · `job` · `ask`, plus `memory` and
`skill_load` when configured. No per-CLI tools: `gh`, `jq`, `rg` and friends are
skills that teach `bash`.

- Memory notes are listed in the prompt by title, up to 40, so the model knows
  what to search for. Bodies load only on a search; `/reload` picks up new notes.
- Results over 4 KB spill to a file; you get a head, a tail, and the path.
- `read` takes several paths; a directory path lists names to three levels, skipping build and VCS noise.
- `edit` replaces exact strings, several per call, all or nothing.
- Long commands run with `background`, then `job wait` (55 s at a time).
  `delay_s` starts one later (up to 24 h), and `job list` shows each job's
  elapsed time, output size, and last output line, so a copy run with
  `rsync --info=progress2` shows how far it has got.
- `show` on `bash` puts a command's output before you with its colours, such
  as a `chafa --format symbols` picture or a coloured chart; the model gets the
  text, or only a picture's size. Commands you type with `!` show in colour too.
  Only colour and style codes pass: cursor moves, clears, and image protocols
  such as sixel and kitty are dropped.
- In the chat, the agent can check back without you: `job check` with
  `after_s` wakes the session later with the job's state, and `notify` wakes it
  when a job ends. The session then reports like any reply, so out of view it
  joins the Ctrl+O queue. At most four check-ins wait per session, none sooner
  than 30 s, and they end with the session. Line mode refuses them.
- `fetch` makes one GET from your machine, local network included. HTML comes
  back as compact markdown (headings, links, lists, tables; menus, banners, and
  footers dropped), RSS and Atom as a list of title, date, and link, and JSON
  and text as they are. Reading never asks in `normal`, nor does a plain
  `curl` GET piped into filters such as `jq`; only a URL over 300 characters
  does, since a long URL can carry data out. `strict` asks every time and
  `yolo` never.
- `fetch` also calls APIs: `method` (POST, PUT, PATCH, DELETE), `headers`, and
  a `body`, sent as JSON when it is an object. Credentials live in the config,
  bound to one host, and come from the environment when the request goes out:
  `auth = api.github.com Authorization: Bearer $GITHUB_TOKEN`. The model never
  sees the key, it is never sent to another host or through a redirect, and the
  variable is treated as a secret. In `normal`, every write asks, naming the
  method, path, and body size, and a credentialed read asks once per host.
- The prompt carries today's date, so questions about news, prices, results,
  and releases search for the current year.
- `search` returns summaries with URLs. It exists only on DeepSeek.

## Configuration

`~/.config/imp/config`, `key=value`; environment wins, except `api_key`, which
wins when set.

```
# provider=deepseek
# model=deepseek-flash
# base_url=https://api.deepseek.com
# permissions=normal        # normal | strict | yolo
# allow_sudo=false
# allow=npm test          # one line per shape; written by `a` at a prompt
# auth=api.github.com Authorization: Bearer $GITHUB_TOKEN   # one line per host
# render.mermaid=mmd      # draw a reply's ```mermaid blocks with a command (stdin, COLUMNS)
# verbosity=medium        # low: a line per step · medium: and its result · high: and the output
# thinking=false
# effort=default          # low | medium | high | default
# compact_at=60000
# color=auto
# mouse=on
# goal_rounds=5         # turns a /goal may run, 1 to 20
# suggest=on           # a greyed next message after each reply; Tab takes it
# diffs=on             # changed lines under each edit or write
# spinner=circle       # braille dots line arc square triangle pulse bar star random
# auto_next=on          # move to the next waiting session after you reply
```

Keys come from the provider's variable and are never written to the config by
the agent.

### Replies

Replies render as markdown: headings, emphasis, inline and fenced code, lists,
quotes, rules, links, and tables. Tables are drawn with box lines and shrink
their widest columns to fit; a table that cannot fit shows each row as
`header: value` lines. Line mode prints the markdown as written. Control
characters and escape sequences from the model or a tool are removed before
anything is shown.

### Mouse

The chat captures the mouse. The wheel scrolls the conversation rather than
the terminal's scrollback. A click on a picker row, a `[y] yes` chip, a
question option, or a picker hint does what its key does; `[a] always` takes
a second click. Every click has a key.

Each reply and code block shows a dim `⧉` at its right edge. Click it, or
run `/copy` or `/copy code`, to copy. Ctrl+V pastes the copy into the
composer. The copy also goes to the terminal clipboard over OSC 52, which
iTerm2, Ghostty, kitty, and WezTerm accept and Terminal.app ignores; under
tmux it needs `set -g set-clipboard on`.

Drag across text to select it; letting go copies it, as the `⧉` label does.
Ctrl+G hands the mouse to the terminal until the next key, for its own
selection. `mouse=off` gives the mouse back for good.

### Themes

`/theme` picks one of eleven: `default`, `default-light`, `solarized-dark`,
`solarized-light`, `gruvbox-dark`, `gruvbox-light`, `catppuccin-mocha`,
`catppuccin-latte`, `tokyo-night`, `nord`, and `mono`, which turns colour off.
The choice is saved. Until you choose, imp picks `default` or `default-light`
from the terminal background, read from `COLORFGBG` or asked of the terminal.

Each theme colours eleven roles: replies, your messages, muted text, borders,
accents, tool rows, success, errors, inline code, links, and the highlighted
picker row. Terminals without truecolor get the nearest 256-colour match.

### Providers

`provider` chooses a profile. `base_url`, `model`, and `api_key` override it.

| provider | endpoint | default model | key |
|---|---|---|---|
| `deepseek` | `https://api.deepseek.com` | `deepseek-flash` | `DEEPSEEK_API_KEY` |
| `openai` | `https://api.openai.com/v1` | `gpt-4.1-mini` | `OPENAI_API_KEY` |
| `anthropic` | `https://api.anthropic.com` | `claude-opus-5-5` | `ANTHROPIC_API_KEY` |
| `gemini` | `https://generativelanguage.googleapis.com` | `gemini-2.5-pro` | `GEMINI_API_KEY` |
| `ollama` | `http://localhost:11434/v1` | `llama3.2` | none |
| `custom` | `http://localhost:8000/v1` | `local-model` | none |

```sh
export OPENAI_API_KEY=...
imp --provider openai
imp --provider ollama --model llama3.2
```

Or put `provider=openai` in the config. `IMP_PROVIDER` does the same.

Key order is config `api_key`, then `IMP_API_KEY`, then the profile's variable.
A missing key names the variable it looked for.

Only DeepSeek implements the `search` endpoint, so other providers get no
search tool and no search line in the prompt; `fetch` works everywhere. Prices are DeepSeek-only:
other providers show tokens and no cost.

### Permissions

| mode | reads | writes | shell |
|---|---|---|---|
| `normal` | allow | allow in the directory | ask unless safe-listed |
| `strict` | allow | ask in the directory | ask |
| `yolo` | allow | allow, jail off | allow |

File tools reach only the session directory, imp's config folder, and its
output folder; other paths are refused without a prompt. `yolo` turns this
jail off.

A safe-listed command such as `ls`, `rg`, or `git diff` runs unprompted only
when the whole line is read-only: a pipe of safe commands is fine, but `&&`,
`;`, redirects, `$(…)`, and flags that write or run things, such as
`find -exec` or `sort -o`, ask. `awk` always asks.

A command prompt reads `run it?   [y] yes  [n] no  [s] session  [a] always`.
In `normal`, `s` lets the same command shape run without asking for the rest
of the session, so `npm test` covers `npm test -- --watch` but not
`npm install`. `a` does the same for every session and appends
`allow = npm test` to the config; delete the line to undo it. Pipes, `&&`,
`;`, and redirects never offer `s` or `a`, and `strict` ignores `allow`.

The agent's commands and jobs run without secret-looking variables (names
ending `_KEY`, `_TOKEN`, `_SECRET`, or `_CREDENTIALS`, any `PASSWORD`, and
`AWS_*`). Pass one through with `pass_env = GH_TOKEN, NPM_TOKEN`. Secret values
that show up in any tool output are replaced with `[redacted NAME]`, and a
command naming a stripped variable asks first. Commands you type with `!`, and
`/shell`, keep your full environment.

Switch for the session with `/mode`; the config and `--permissions` set the
starting mode.

Denied in every mode, `yolo` included: `dd`, `mkfs`, shutdown, `rm -rf` outside the
directory, `chmod -R /`, force push, hard reset, downloads piped to a shell.
`sudo` needs `allow_sudo=true`.

These are guardrails, not a sandbox the agent cannot leave.

### Files it reads

```
AGENTS.md                     global, repository root, then session directory
.claude/CLAUDE.md             fallback when AGENTS.md is absent
~/.config/imp/skills/<n>/SKILL.md
./.imp/skills/<n>/SKILL.md
./.agents/skills/<n>/SKILL.md
~/.config/imp/agents/<n>.md
./.imp/agents/<n>.md
./.agents/agents/<n>.md
```

Project sources shadow global ones by name. `.agents` shadows `.imp`, and an
explicit `--skills-dir` wins over both.

Agent frontmatter: `description`, `tools`, `model`, `thinking`, `effort`. Skills use
`name` and `description`; only the description reaches the prompt until loaded.

## Sessions

One JSONL transcript plus a small sidecar per session under
`~/.local/state/imp/sessions/`. Resume restores reasoning, tool calls, tool
results and the compaction summary, so the model continues with real context.
`--list`, `--rename`, `--delete` work headlessly; delete moves to trash.

Job logs live under `~/.local/state/imp/out/jobs/`. They are kept for seven
days, and the output directory is capped at 64 MB; legacy spill files are
removed at startup.

## Development

```sh
cargo test
openspec validate --all --strict
./scripts/size.sh   # size report; the goal is as small as possible
```

This repository uses OpenSpec: source changes need an active change. See
`AGENTS.md` and `openspec/`.

## Limits

- Reads the web, does not browse it: no JavaScript, no logins, no clicking.
- Compaction summarises; it can lose detail. Durable facts belong in memory.
- Jobs do not survive a restart.
- `sudo` with an uncached password is not supported yet.
