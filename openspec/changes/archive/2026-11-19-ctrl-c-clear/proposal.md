# Proposal

## Why

Ctrl+C always exits, even when a half-written prompt sits in the composer.
A mistaken keypress loses the draft and the session, when clearing the draft
would have been enough. An empty composer still needs an exit that works from
any state.

## What Changes

- Ctrl+C with text in the composer clears that text and leaves the session
  running; the draft is not sent.
- Ctrl+C with an empty composer keeps today's behaviour and exits.
- With a picker open, Ctrl+C clears the input and closes the overlay before it
  considers exiting.
- Confirmation and `ask` prompts keep their escape: Ctrl+C exits, since those
  are not composer text.
- Streaming is treated the same way: clear a draft if there is one, otherwise
  exit and end the turn.

## Capabilities

### Modified Capabilities

- `tui`: the keyboard requirement states the two-way Ctrl+C rule.

## Impact

- `src/tui.rs`: key routing, tests.
- `src/help.rs` and `README.md`: the key line.
- `openspec/specs/tui/spec.md`.
