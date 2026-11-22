# Proposal

## Why

On a cheap model the status line reads `0 tok · $0.00` for a whole turn. The
context estimate is only sent when a turn ends, and the cost is rounded to two
decimals, so a real $0.001 is drawn as `$0.00`.

## What Changes

- Every request's usage carries the running context estimate, so the token
  figure moves during a turn and not only when it ends.
- The cost shows three decimals below $0.10 and two above, matching the task
  line.
- Tests for the small-cost format and for a mid-turn token update.

## Keyboard and resources

None. One extra number in each usage event.

## Capabilities

### Modified Capabilities

- `tui`: the status line's cost format and refresh.
