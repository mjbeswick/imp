# Proposal

## Why

On the Hackberry keyboard, the most common next messages, such as "yes",
"run the tests", or "commit it", still have to be typed out. Claude Code
offers the likely next prompt greyed out in the input, and one key accepts it.

## What Changes

- After a turn ends, the worker makes one small request for the user's likely
  next message, from the same history so the provider's prompt cache applies.
- The chat draws it greyed out in an empty input. Tab or Right fills it in
  for editing or sending; typing anything drops it.
- It is not added to the conversation or the log. Its tokens count toward the
  status line's cost.
- `suggest = on | off` in the config, on by default. Line mode never asks.

## Capabilities

### Modified Capabilities

- `agent-loop`: a suggestion request after each turn.
- `tui`: the greyed suggestion and how it is accepted.
