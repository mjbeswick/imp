# mote

A small terminal agent for DeepSeek. One binary, a chat interface, and a
deliberately short list of tools.

```
mote: 560 KB · ~8 MB RSS · 0 runtime dependencies
```

## Install

```sh
cargo build --release
install -m 755 target/release/mote ~/.local/bin/mote
mote init
export DEEPSEEK_API_KEY=...
```

`mote init` creates `~/.config/mote/` and `~/.local/state/mote/`, writes a
commented config and one example agent and skill, and overwrites nothing.

Needs libcurl at runtime: present on macOS and on any Linux with curl installed.

## Use

```sh
mote                                  # full-screen chat
mote "why is the build failing?"      # one request, then exit
mote -c                               # continue the last session
mote -r 3k9a0aa                       # resume a specific session
printf 'ls\n/quit\n' | mote           # scripted line mode
```

Both front-ends share the session, tools, and history. Line mode is chosen when
stdin or stdout is not a terminal, and prints no escape sequences.

### Keys

```
Enter send · Alt+Enter newline · Esc cancel · Ctrl+C quit · Ctrl+L clear
Up/Down history · Tab complete
```

### Commands

```
/help      keys and commands     /agent     switch profile
/new       fresh conversation    /reload    re-read instructions and skills
/sessions  list and pick         /compact   compact now
/think     thinking toggle       /quit      leave
```

A line starting with `!` runs as a shell command, unchecked by the model, with
the same limits as the `bash` tool.

## Tools

`bash` · `read` · `edit` · `write` · `search` · `job` · `ask`, plus `memory` and
`skill_load` when configured. No per-CLI tools: `gh`, `jq`, `rg` and friends are
skills that teach `bash`.

- Results over 4 KB spill to a file; you get a head, a tail, and the path.
- `edit` replaces exact strings, several per call, all or nothing.
- Long commands run with `background`, then `job wait` (55 s at a time).
- `search` returns summaries; `fetch` returns one page with its outline.

## Configuration

`~/.config/mote/config`, `key=value`; environment overrides everything.

```
# model=deepseek-flash
# permissions=normal        # normal | strict | yolo
# allow_sudo=false
# thinking=false
# compact_at=60000
# color=auto
```

`DEEPSEEK_API_KEY` is required and never written to the config by the agent.

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
~/.config/mote/skills/<n>/SKILL.md
./.mote/skills/<n>/SKILL.md
~/.config/mote/agents/<n>.md
./.mote/agents/<n>.md
```

Agent frontmatter: `description`, `tools`, `model`, `thinking`. Skills use
`name` and `description`; only the description reaches the prompt until loaded.

## Sessions

One JSONL transcript plus a small sidecar per session under
`~/.local/state/mote/sessions/`. Resume restores reasoning, tool calls, tool
results and the compaction summary, so the model continues with real context.
`--list`, `--rename`, `--delete` work headlessly; delete moves to trash.

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
