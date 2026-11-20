# Proposal

## Why

The approval row is one colour of run-on text, so the answer keys are hard to
pick out at a glance. And an always lasts one session: the same `npm test`
asks again in every new session, forever.

## What Changes

- The prompt draws key chips: `run it?   [y] yes  [n] no  [s] session  [a] always`
  with the key letters bold in the accent colour, the labels muted, and the
  countdown muted at the right edge.
- `s` is today's `a`: the command shape is approved for this session.
- `a` approves the shape for good: imp appends `allow = <shape>` to its config
  file and applies it to every live session at once.
- In `normal`, a command whose shape matches an `allow` line runs without
  asking. `strict` still asks every time; the deny list and sudo rules still
  win.
- `s` and `a` are offered exactly where `a` is offered today. The inbox and
  line mode accept `s` too.

## Capabilities

### Modified Capabilities

- `permissions`: a saved allow list beside the session always.
- `tui`: key chips on the confirmation row, `s` and `a`.
- `inbox`: `s` answers from the inbox.

## Impact

- `src/config.rs`, `src/agent.rs`, `src/tui.rs`, `src/main.rs`, `README.md`.
