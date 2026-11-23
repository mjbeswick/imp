# Proposal

## Why

The active agent shows up as an `agent: plan` transcript line and in the
status line, neither where the eye is when typing. The rule above the
composer repeats the send mode that the prompt glyph already shows.

## What Changes

- The composer starts with the agent the next message goes to: `plan >`.
  During a turn, after Tab, that is the coming agent.
- With one agent, no name: `>` alone.
- No `agent:` transcript line on a switch, and no agent in the status line.
  Mode changes keep their line.
- The rule loses `steer · ^Q queue`; `>` and a muted `»` remain the mode.

## Keyboard and resources

No keys change. The name costs its length plus one column of composer width
on the Hackberry, and only while several agents exist.
