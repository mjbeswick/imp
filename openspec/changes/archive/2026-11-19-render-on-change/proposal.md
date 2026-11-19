# Proposal

## Why

The full-screen interface repaints itself every 120 ms whether or not anything
changed. On the Hackberry Pi that is 4,116 bytes a second of identical frames,
and under kmscon's software renderer the screen visibly flickers while the prompt
sits idle.

The timer existed to keep the streaming spinner moving, but the key-read timeout
already redraws while a turn runs, so the unconditional clause only costs work.

## What Changes

- Render only when a key was handled, an event arrived, or the app is dirty.
- Throttle the streaming spinner to ten frames a second instead of twenty.
- Nothing else about layout, layout timing, or streaming output changes.

## Capabilities

### Modified Capabilities

- `tui`: the interface redraws on change, not on a timer.

## Impact

- `src/tui.rs`: the main loop's render condition and the spinner tick.
- `openspec/specs/tui/spec.md`: a scenario for an idle screen.
