# Proposal

## Why

A confirmation or question gives up after 60 seconds, refuses, and stops the
turn. That only ever loses work: a refusal is what waiting gives anyway, and
the user who stepped away comes back to a stopped turn. The countdown pushes
for a quick approval and redraws the screen every second.

## What Changes

- Confirmations and `ask` questions wait until answered. No deadline, no
  refusal on time, no stopped turn.
- The seconds-left countdown leaves the confirmation row.
- Yes stays the default: `y` and Enter approve.
- With a prompt open and nothing moving, the loop sleeps as long as when idle.

## Keyboard and resources

No keys change. One fewer redraw per second while a prompt waits, which
matters on the Pi Zero.

## Capabilities

### Modified Capabilities

- `tui`: confirmation rows lose the deadline and countdown.
- `inbox`: sessions out of view no longer need a deadline exemption.
