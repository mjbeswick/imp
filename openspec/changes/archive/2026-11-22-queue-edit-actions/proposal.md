# Proposal

## Why

A queued message can only be edited or emptied. Changing your mind about how
it should arrive, or dropping it, means emptying it and retyping it
elsewhere. The queue also stalls when you leave an open message with ↑ after
the turn has ended, because only Enter and ↓ release it.

## What Changes

- `Ctrl+Q` on an open queued message sends it now: into the running turn as a
  steer, or as a new turn when none runs. It leaves the queue either way.
- `Ctrl+X` deletes the open message and leaves the next one open, so several
  delete in a row cost one key each.
- `Ctrl+C` closes the editor and leaves that message queued as it was.
- Every way out of the editor releases the queue when the turn ended while it
  was open.
- The rule names the keys and says `steer now` or `send now`.

## Keyboard and resources

`Ctrl+Q` and `Ctrl+X`, shown in the rule while a queued message is open. No
new state beyond the editor that already exists.

## Capabilities

### Modified Capabilities

- `tui`: actions on an open queued message, and releasing the queue on exit.
