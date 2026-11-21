# Proposal

## Why

imp works until the model stops calling tools, then waits for the user. For
work with a clear finish line, such as "all tests pass" or "the build is
green on the Pi", the user has to keep typing "continue". Claude Code's
`/goal` keeps the agent working across turns until the goal is met.

## What Changes

- `/goal <text>` sets a goal and starts a turn on it. `/goal` shows it and
  its round; `/goal off` clears it.
- After each turn, if the reply does not end with `GOAL MET` or
  `GOAL BLOCKED: <why>`, imp starts another round with a note restating the
  goal, up to `goal_rounds`, which defaults to 5.
- The goal ends when it is met or blocked, when the rounds run out, on Esc,
  when the user sends a message, when a confirmation is declined, or on an
  error. The status line shows `goal 2/5` while one runs.
- Chat only, as check-ins are.

## Capabilities

### Modified Capabilities

- `agent-loop`: goal rounds and their ends.
- `tui`: the command and the status line.
