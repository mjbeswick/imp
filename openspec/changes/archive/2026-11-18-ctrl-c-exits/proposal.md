# Proposal

## Why

Ctrl+C already exits when the session is idle, but the spec still promises a
two-press guard that no longer exists, and during a running turn every press
only cancels. A user who wants out mid-answer has no press that exits.

## What Changes

- Idle: one Ctrl+C exits.
- Running turn: the first press cancels, the second exits, even while the turn
  is still unwinding.
- The status row says `cancelling · ctrl+c again to exit` after the first press.
- The flag clears when the turn ends, so the next turn starts calm.
- The keyboard requirement and its quit scenario describe this, not a two-press
  guard.

## Capabilities

### Modified Capabilities

- `tui`: Ctrl+C exits, cancels, then exits.

## Impact

- `src/tui.rs`: key handling and one status line.
- `openspec/specs/tui/spec.md`: keyboard requirement.
