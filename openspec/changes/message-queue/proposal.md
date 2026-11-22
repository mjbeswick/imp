# Proposal

## Why

A message sent while a turn runs goes straight to the worker: it shows in the
transcript at once, cannot be edited or held back, and restarts the running
turn's timer and step count. There is no way to steer a running turn, and
stashing a draft needs a typed command.

## What Changes

- A message sent during a turn waits in a queue drawn dim above the
  composer, and is sent when the turn ends, oldest first.
- Up on an empty draft pulls the newest queued message back for editing and
  pauses the queue. Enter puts it back and resumes.
- Ctrl+X sends the draft into the running turn, read before the model's next
  step. With no turn running it sends as Enter does.
- Ctrl+S stashes the draft, or pops the newest stash into an empty draft.
- A message sent during a turn no longer resets the turn's meter.

## Keyboard and resources

New keys: Ctrl+X and Ctrl+S; Up gains a meaning while messages wait. Raw
mode already clears flow control, so Ctrl+S reaches imp. The queue lives in
memory per session.

## Capabilities

### Modified Capabilities

- `tui`: a message queue, steering, and a stash key.
- `agent-loop`: steering messages join a running turn.
