# Proposal

## Why

A message sent while a turn runs goes straight to the worker: it shows in the
transcript at once, cannot be edited or held back, and restarts the running
turn's timer and step count. A running turn cannot be steered. Stash is a
second list for the same job.

## What Changes

- Enter sends when idle and steers a running turn: the message joins the
  turn before the model's next step.
- Alt+Enter, or Ctrl+Q in any terminal, queues the draft. Queued messages
  show dim above the composer and are sent in order as turns end.
- ↑/↓ from an empty draft move through queued messages, then history. A
  queued message opens in the composer for editing; Enter saves it in its
  place, and saving it empty deletes it.
- While a queued message is open, the running turn waits before its next
  step and carries on when it is saved or left.
- `/queue` replaces `/stash`: list, `pop n`, `delete n`, `clear`. A queue
  with nothing running is what the stash was.
- Shift+Enter, or Ctrl+J in any terminal, inserts a newline; Alt+Enter no
  longer does.
- A message sent during a turn no longer resets the turn's meter.

## Keyboard and resources

Ctrl+Q reaches imp because raw mode clears flow control. Alt+Enter needs
"Option as Meta" on macOS terminals, so Ctrl+Q is shown beside it. Shift+Enter
needs the kitty keyboard protocol, so Ctrl+J is shown beside it.

## Capabilities

### Modified Capabilities

- `tui`: keys, the queue, and `/queue` for `/stash`.
- `agent-loop`: steering and pausing a running turn.
