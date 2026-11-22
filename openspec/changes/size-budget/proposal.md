# Proposal

## Why

The binary grew from 572 KB to 972 KB with nothing stopping it.
`scripts/size.sh` reports growth but never fails, by design: "the goal is as
small as possible, not a fixed limit". This change reverses that choice.

## What Changes

- `scripts/size.sh` fails when the release binary exceeds a recorded limit.
- Raising the limit is an explicit edit, so growth is a decision.
- Optional, after `pi-release`: release builds rebuild std with size
  settings, measured at -15%.

## Keyboard and resources

No keys change. A smaller binary loads faster on the Pi.
