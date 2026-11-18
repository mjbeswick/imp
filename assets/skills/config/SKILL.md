---
name: config
description: Change imp's own configuration, skills, agents, or memory. Use when asked to add a skill, create an agent, remember a preference, or explain where config lives. Cannot edit the main config file.
---

# Configuring imp

## Layout

```
~/.config/imp/config          settings, key=value, key stays in the environment
~/.config/imp/skills/<n>/SKILL.md   global skills
~/.config/imp/agents/<n>.md         global agent profiles
~/.local/state/imp/sessions/        one JSONL transcript per session
~/.local/state/imp/memory/<n>.md    global notes
~/.local/state/imp/out/             spilled tool output and fetched pages

./.imp/skills/<n>/SKILL.md     project skills, shadow global ones by name
./.imp/agents/<n>.md           project agents
./.imp/memory/<n>.md           project notes
```

`imp init` creates the directories and one example of each file.

## Skills

One directory per skill, one `SKILL.md` inside. Frontmatter needs `name` and a
`description` written for routing: the words someone would use when they want
the skill, plus the tool names involved.

```markdown
---
name: jq
description: Query and reshape JSON with jq. Use for jq filters, extracting fields, formatting JSON output.
---

# jq

- Read: `jq '.items[] | {id, name}' file.json`
- Never edit files in place without a backup.
```

The body is loaded only when the skill is selected, so it can be detailed.

## Agents

One markdown file per profile. The body is the instructions; frontmatter is
optional and takes `description`, `tools`, `model`, `thinking`.

```markdown
---
description: reviews diffs and reports problems
tools: read, bash
thinking: true
---

You review code. Report only defects, with file and line.
```

Switching agents keeps the conversation and appends a note that supersedes the
previous instructions.

## Memory

One note per markdown file. The filename is the title; the first heading is the
title as read back. No frontmatter. Search matches filename and body.

## Rules

- Do not modify `~/.config/imp/config`. It is refused: permission changes and
  model settings belong to the user, not to you.
- Ask before adding a skill or agent, and say what it will route on.
- Changes are files; there is no daemon to restart. Instructions and skills are
  read at startup, so tell the user to restart for them to take effect.
