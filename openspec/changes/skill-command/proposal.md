# Proposal: /skill command

## Why

Skills are loaded only by the model, through `skill_load`. The user can switch skills on or off with
`/skills`, but cannot choose a skill for a message. A user who knows which skill a task needs has to hope
the model loads it.

## What Changes

- `/skill <name>` queues an enabled skill. Its full instructions go to the model with the next message.
- The skill is not shown in the transcript as part of the user's message. The model receives it.
- A queued skill is sent once, with that message. It is not kept for later messages.
- `/skill <name>` for an unknown or switched-off skill shows an error and queues nothing.
- Bare `/skill` shows the usage line and lists the enabled skills.
- The command is listed in the command table, so `/help` and the command palette show it.

## Out of Scope

- A picker for skills. Bare `/skill` lists them as text.
- Bare `/<name>` commands for skills, which could clash with built-in commands.
- Changing `skill_load` or the 4 KB cap on a skill's instructions.

## Impact

- `src/tui.rs`: the `/skill` command, and a command to the worker that queues the skill.
- `src/help.rs`: the command table entry.
- `src/tui/tests.rs`: tests for queuing, the error case and the usage line.
- `README.md`: the skills paragraph.
