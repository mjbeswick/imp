# Proposal

## Why

A custom answer to an `ask` question needs `e` first. People type straight
away, and the letters vanish. `y` and `n` pick on any two-choice question, so
a typed `yes, but…` would send the first option.

## What Changes

- Typing a letter on a question opens the answer row with that letter typed.
- Digits still pick a choice. Esc still stops.
- `y` and `n` pick only when the choices are Yes and No.
- Enter on an empty answer does nothing.
- The hint reads `type to answer   Esc stop`. The `[e]` key is gone.
- The spec drops the arrow promise for the custom row; no arrow reaches it.

## Keyboard and resources

No new keys. No new process.

## Capabilities

### Modified Capabilities

- `tui`: typing starts a custom answer.
- `tools`: `y`/`n` apply to Yes/No questions only.
