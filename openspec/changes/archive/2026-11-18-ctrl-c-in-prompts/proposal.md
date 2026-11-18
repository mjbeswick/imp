# Proposal

## Why

Ctrl+C exits when the session is idle, but not while a prompt is open. The key
handler hands every key to the prompt when one is pending, and the picker,
question, and rename arms have no Ctrl+C case, so the key is dropped. A user
with `/sessions` open has no way out except Esc then Ctrl+C.

## What Changes

- Ctrl+C exits from any pending prompt: session picker, agent picker, `ask`
  question, rename, delete confirmation.
- Ctrl+C on a command confirmation keeps its current meaning: refuse the call,
  stop the turn. That prompt is a safety gate, and a stray Ctrl+C should not kill
  the session mid-decision.
- The keyboard requirement gains a scenario for the picker case.

## Capabilities

### Modified Capabilities

- `tui`: Ctrl+C exits from an open prompt.

## Impact

- `src/tui.rs`: key dispatch order and two tests.
- `openspec/specs/tui/spec.md`: keyboard requirement.
