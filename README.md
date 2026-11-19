# imp

A small terminal agent for DeepSeek, OpenAI, Anthropic, Gemini, and local
models. One binary, a chat interface, and a deliberately short list of tools.

```
572 KB binary · 6 ms cold start · 6.2 MB idle RSS · 0 runtime dependencies
```

Measured on an M1 MacBook Air, release profile `lto = "fat"`, `opt-level = "z"`,
`panic = "abort"`, stripped. Cold start is the median of 30 runs of `imp
--version`; RSS is the full-screen interface idle for two seconds.

## Install

```sh
cargo build --release
install -m 755 target/release/imp ~/.local/bin/imp
imp init
export DEEPSEEK_API_KEY=...
```

`imp init` creates `~/.config/imp/` and `~/.local/state/imp/`, writes a
commented config and one example agent and skill, and overwrites nothing.

Needs libcurl at runtime: present on macOS and on any Linux with curl installed.

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
Ctrl+O expand the last tool call · Ctrl+D leaves line mode
```

### Commands

```
/help      keys and commands     /agent     switch profile
/new       fresh conversation    /reload    re-read instructions and skills
/sessions  pick a session        /compact   compact now
/think     thinking toggle       /yolo      prompt-free for this session
/verbosity level: low or high    /cwd       show the session directory
/cd        browse or change cwd  /image     browse or attach an image
/fork      copy the session      /theme     set the colour theme
/shell     open a shell          /model     set or pick the session model
/provider  manage providers      /stash     park a message for this session
/rename    rename this session   /quit      leave
```

`/agent`, `/model`, `/theme`, `/verbosity`, `/provider`, and `/sessions` are
value commands. Bare, each opens a centred picker on the current entry, marked
`current`; a typed value such as `/theme mono` or `/sessions 51280befc` applies
directly. Line mode prints the same values as aligned rows. The filter has the
cursor: type to narrow, move with Up/Down or PgUp/PgDn, choose with Enter, and
cancel with Esc. A space hands the line back to the composer, so
`/provider add work` types straight through.

Tab in the command list completes the highlighted command: a value command
opens its picker, `/stash` and `/rename` get a space for their text, and any
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

`/stash <text>` parks a message, bare `/stash` lists them, `pop [n]` loads one
into the composer, `delete n` removes one, and `pop all` loads every entry.
Stashes belong to the session and never reach the model until you send them.
`/rename <title>` retitles the current session.

A line starting with `!` runs as a shell command, unchecked by the model, with
the same limits as the `bash` tool.

## Tools

`bash` · `read` · `edit` · `write` · `search` · `job` · `ask`, plus `memory` and
`skill_load` when configured. No per-CLI tools: `gh`, `jq`, `rg` and friends are
skills that teach `bash`.

- Results over 4 KB spill to a file; you get a head, a tail, and the path.
- `read` takes several paths; a directory path lists names to three levels, skipping build and VCS noise.
- `edit` replaces exact strings, several per call, all or nothing.
- Long commands run with `background`, then `job wait` (55 s at a time).
- `search` returns summaries; `fetch` returns one page with its outline. Both
  exist only on DeepSeek.

## Configuration

`~/.config/imp/config`, `key=value`; environment wins, except `api_key`, which
wins when set.

```
# provider=deepseek
# model=deepseek-flash
# base_url=https://api.deepseek.com
# permissions=normal        # normal | strict | yolo
# allow_sudo=false
# thinking=false
# compact_at=60000
# color=auto
# mouse=on
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

The chat captures the mouse wheel, so it scrolls the conversation rather than
the terminal's scrollback. Clicks are ignored; select text with your terminal's
modifier, Option-drag in iTerm. `mouse=off` gives the mouse back to the
terminal.

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
| `anthropic` | `https://api.anthropic.com` | `claude-sonnet-4-5` | `ANTHROPIC_API_KEY` |
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

Only DeepSeek implements the `search` and `fetch` endpoints, so other providers
get no search tool and no search line in the prompt. Prices are DeepSeek-only:
other providers show tokens and no cost.

### Permissions

| mode | reads | writes | shell |
|---|---|---|---|
| `normal` | allow | allow in the directory, ask outside | ask unless safe-listed |
| `strict` | allow | ask | ask |
| `yolo` | allow | allow, jail off | allow |

Denied in every mode but `yolo`: `dd`, `mkfs`, shutdown, `rm -rf` outside the
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

Agent frontmatter: `description`, `tools`, `model`, `thinking`. Skills use
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
./scripts/size.sh
```

This repository uses OpenSpec: source changes need an active change. See
`AGENTS.md` and `openspec/`.

## Limits

- Reads the web, does not browse it: no JavaScript, no logins, no clicking.
- Compaction summarises; it can lose detail. Durable facts belong in memory.
- Jobs do not survive a restart.
- `sudo` with an uncached password is not supported yet.
