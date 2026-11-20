# Proposal

## Why

The chat loop wakes twenty times a second even when nothing happens, which
costs battery on a Pi Zero. Two small leftovers from the code review go with
it: an unused theme field and a test that never runs.

## What Changes

- The loop waits 50 ms only while a turn streams, a spinner turns, or within
  two seconds of a key or event; one second while a prompt counts down; five
  seconds otherwise.
- A resize wakes the loop at once through a signal handler, so redraws on
  resize stay immediate.
- `Theme.light` goes; `ctrl_c_exits_at_once_when_idle` gets its `#[test]`.

## Capabilities

### Modified Capabilities

- `tui`: the idle loop sleeps.

## Impact

- `src/tui.rs`, `src/theme.rs`.
