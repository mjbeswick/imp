# Getting started

This guide takes you from a fresh build to a working session. The
[README](../README.md) is the full reference; [The agent loop](agent-loop.md)
explains what happens after you press Enter.

## Install

imp is one binary. It needs Rust to build and libcurl to run, which macOS and
any Linux with curl already have.

```sh
cargo build --release
install -m 755 target/release/imp ~/.local/bin/imp
imp init
```

`imp init` creates `~/.config/imp/` with a commented `config` and one example
skill, and `~/.local/state/imp/` for sessions, memory and output. It never
overwrites anything, so it is safe to run again.

## Your first run

```sh
imp
```

With no API key, imp opens a provider picker. Choose one, then type the key
into the masked row. imp checks it with one request and saves it to
`~/.config/imp/keys`, readable only by you. A wrong key keeps the row open
with the provider's answer. Ollama needs no key and starts at once.

You can also set the key yourself before starting:

```sh
export DEEPSEEK_API_KEY=...      # or OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY
imp --provider anthropic         # or put provider=anthropic in the config
```

## The screen

```
 fix the parser                                  14:05   ← title bar
                                                         
> fix the parser so blank lines are kept                 ← your message
✓ read src/parse.rs → 212 lines                          ← a tool call
✓ edit src/parse.rs → 1 change                           
The parser now keeps blank lines …                       ← the reply
──────────────────────────────────────────────────────── 
build > _                                                ← agent and prompt
deepseek-flash · 3.1K tok · $0.002          ~/project    ← status line
```

- The **title bar** names the session and shows the time. With several
  sessions open, their position comes first, such as `2/3`.
- The **prompt** names the agent your next message goes to.
- The **status line** shows the model, tokens and cost. It also hosts short
  confirmations, such as `copied 42 characters`, for three seconds.

## A first task

Type what you want and press Enter. imp sends it to the model with a small
set of tools: `bash`, `read`, `edit`, `write`, `fetch`, `search`, `job` and
`ask`. Each tool call shows as one row while it runs.

Some actions ask first. A command prompt looks like this:

```
run it?   [y] yes  [n] no  [s] session  [a] always
```

`s` allows that command shape for the rest of the session; `a` allows it in
every session and saves an `allow =` line in the config. `n` declines, and
the model is told you declined.

While the model works:

- **Esc** stops the turn.
- **Enter** sends a message into the running turn; **Ctrl+Q** switches to
  queueing it for after the turn instead. The prompt glyph shows which:
  `>` steers, a muted `»` queues.
- **Ctrl+R** sends your last message again and drops the reply.
- **Ctrl+E** expands the latest tool rows to show their commands and output.

If the model asks you a question, press a number to pick a choice, or start
typing to give your own answer.

## Agents and modes

Three agents are built in. **Tab** on an empty draft moves between them:

| Agent | Tools | For |
|---|---|---|
| `build` | all | making the change |
| `plan` | read-only | investigating and writing a plan |
| `review` | read-only | finding defects in a diff |

A file at `~/.config/imp/agents/plan.md`, or `.agents/agents/plan.md` in a
project, replaces the built-in one with the same name.

**Shift+Tab** steps the permission mode: `normal` asks before commands that
are not plainly read-only, `strict` asks before every change and command, and
`yolo` asks for nothing. File tools never leave the project folder except in
`yolo`, and some commands are refused in every mode.

`/think` turns thinking on or off, and `/effort low|medium|high` sets how hard
the model works where the provider supports it.

## Everyday keys

| Key | Does |
|---|---|
| Ctrl+P | the palette: every command and key action in one list |
| `?` | the keys card, from an empty draft |
| `/` | the command list |
| Up, Down | queued messages, then your history |
| Shift+Enter, Alt+Enter, Ctrl+J | a new line |
| Ctrl+C | clear the draft, else stop the turn; twice to quit |
| PgUp, PgDn or the wheel | scroll |
| drag | select and copy; Ctrl+V pastes |

## Sessions

Every session is saved as you go. `imp -c` continues the last one,
`imp -r <id>` a specific one, and `/sessions` lists them all.

`/new` opens another live session beside the current one, up to four.
**Ctrl+N** moves between them. **Ctrl+O** jumps to the one that has waited
longest for you: a question, an approval, or a reply that finished out of
view.

## Without the chat

```sh
imp "why is the build failing?"      # one request, then exit
printf 'ls\n/quit\n' | imp           # scripted: line mode, no escape codes
```

## Teaching it your project

- `AGENTS.md` in a repository is read at the start of every session there.
- Skills are folders with a `SKILL.md` under `~/.config/imp/skills/` or
  `.agents/skills/`. Only their descriptions reach the prompt until the
  model loads one.
- `/reload` picks up edits to the config, instructions and skills without a
  restart.

## On a Pi or another machine

`scripts/deploy.sh host…` cross-builds on your machine and installs over SSH,
keeping the previous binary as `imp.prev`. `scripts/measure.sh host` reports
the size, start-up time and memory there.

imp has no daemon. To keep sessions running through a dropped connection,
start it under tmux:

```sh
ssh -t host tmux new -As imp imp
```

## Next

- [The agent loop](agent-loop.md): what a turn does, step by step.
- [README](../README.md): every command, key, setting and limit.
