# Proposal

## Why

Ctrl+C has three meanings today. Idle it exits, during a turn it cancels and
needs a second press, and on a confirmation it refuses the command and keeps
running. Two of those do not exit, which reads as a broken key.

Esc already cancels a turn and `n` or Esc already refuses a prompt, so the other
meanings are covered.

## What Changes

- Ctrl+C exits from every state: idle, streaming, tool call, confirmation,
  picker, question, rename.
- The cancel-then-exit latch and its status line go away.
- Esc keeps cancelling a turn. `n` and Esc keep refusing a prompt.

## Capabilities

### Modified Capabilities

- `tui`: Ctrl+C always exits.

## Impact

- `src/tui.rs`: one guard, remove the latch.
- `openspec/specs/tui/spec.md`: keyboard requirement.
