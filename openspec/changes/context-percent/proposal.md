# Proposal

## Why

The status line shows the estimated context as a token count, such as `12.3K tok`, but not how full the model's context window is. A user cannot tell at a glance how close a long session is to the limit.

## What Changes

- The status line shows the share of the context window in use, in whole percent, after the token count: `12.3K tok (6%)`.
- The window size comes from a per-model table, or from a new `context_window` config key that overrides the table for any model.
- When the window size is unknown, the percentage is omitted and the token count is shown alone, as today.
- The percentage is capped at 100% and refreshes after each request, with the token count.

## Capabilities

- **Modified:** `tui` (the status line requirement "The status line reports tokens and cost").
- **New:** none.

## Out of Scope

- Warnings or colour changes at a threshold.
- Changing how the token estimate is computed.
- Automatic compaction when the window fills.

## Impact

- `src/tui.rs` status line rendering.
- `src/config.rs` new `context_window` key.
- The window table, in a new module, with a cited source for each entry.
