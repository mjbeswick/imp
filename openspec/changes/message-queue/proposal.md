# Proposal

## Why

A message sent while a turn runs goes straight to the worker: it shows in the
transcript at once, cannot be edited or held back, and restarts the running
turn's timer and step count. A running turn cannot be steered. Stash is a
second list for the same job.

## What Changes

- Enter always sends. When idle it sends; during a turn it steers or queues,
  by a mode that Ctrl+Q toggles. Steer gives the message to the turn before
  the model's next step; queue holds it until the turn ends.
- The mode shows while a turn runs: `>` steer, `»` queue, in the prompt glyph
  and the status line. Steer is the default; the last choice holds for the
  session.
- Queued messages show dim above the composer and are sent in order as turns
  end.
- ↑/↓ from an empty draft move through queued messages, then history. A
  queued message opens in the composer for editing; Enter saves it in its
  place, and saving it empty deletes it.
- While a queued message is open, the running turn waits before its next
  step and carries on when it is saved or left.
- `/queue` replaces `/stash`: list, `pop n`, `delete n`, `clear`. A queue
  with nothing running is what the stash was.
- A message sent during a turn no longer resets the turn's meter.

## Keyboard and resources

Ctrl+Q reaches imp in every terminal because raw mode clears flow control.
Newline keys are unchanged: Shift+Enter, Alt+Enter, and Ctrl+J.

## Capabilities

### Modified Capabilities

- `tui`: keys, the queue, and `/queue` for `/stash`.
- `agent-loop`: steering and pausing a running turn.
