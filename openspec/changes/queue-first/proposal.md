# Proposal

## Why

Enter during a turn steers: the message joins the turn at its next step.
That is right for a correction and wrong for a follow-up, which can derail a
long plan. Typing ahead is the common case. The Ctrl+Q switch between the two
adds a hidden state: the same Enter does different things.

## What Changes

- Enter during a turn queues the message, so the prompt shows `»`.
- Ctrl+S steers, and sends when no turn runs.
- The send mode and its switch go: no Ctrl+Q toggle, no `send_mode` setting,
  one meaning for each key.
- A queued message open for editing sends on Ctrl+S instead of Ctrl+Q, so
  one key means "now".
- The keys card, the palette (`steer now`) and the docs follow.

## Keyboard and resources

Ctrl+S is one chord, easy on the Hackberry's keyboard. imp's raw mode clears
`IXON` on macOS and Linux, so the terminal does not swallow it as XOFF. Ctrl+Q
is freed. No cost.
