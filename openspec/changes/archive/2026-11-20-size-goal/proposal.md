# Proposal

## Why

A hard byte limit gets raised one 16 KB step at a time whenever a feature
needs room, so it stops guiding design. The goal is a binary as small as
possible, with every change accounting for what it costs.

## What Changes

- `scripts/size.sh` prints the binary size and the change from the last
  recorded size, and never fails the build.
- The last recorded size lives in `scripts/size.txt`, updated when a change is
  archived.
- `AGENTS.md` replaces "binary under 776,000 bytes" with the goal: keep the
  binary as small as possible, and state each change's expected bytes in its
  design and its measured bytes when it is archived.

## Impact

- `scripts/size.sh`, `scripts/size.txt`, `AGENTS.md`, `README.md`. No
  behaviour change.
