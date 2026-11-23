# Proposal

## Why

Enter during a turn steers: the message joins the turn at its next step. That
is right for a correction and wrong for a follow-up, which can derail a long
plan. Typing ahead is the common case, and queueing is the predictable
answer to it.

## What Changes

- Enter during a turn queues the message, so the prompt shows `»`.
- Ctrl+S steers once, in either mode, and sends when no turn runs.
- `send_mode = queue|steer` sets the starting mode; Ctrl+Q still toggles it.
- A queued message open for editing sends on Ctrl+S instead of Ctrl+Q, so
  one key means "now".
- The keys card, the palette and the docs follow.

## Keyboard and resources

Ctrl+S is one chord, easy on the Hackberry's keyboard. imp's raw mode clears
`IXON` on macOS and Linux, so the terminal does not swallow it as XOFF; Ctrl+Q,
its pair, already works the same way. No cost.
