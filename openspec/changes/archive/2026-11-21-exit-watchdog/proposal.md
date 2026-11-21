# Proposal

## Why

When the terminal stops reading, as a frozen SSH session does, a frame write
blocks the main thread, and the SIGTERM handler blocks on the same output
writing its terminal reset, so the process cannot be stopped. SIGHUP, which a
dropped connection sends, is not handled at all.

## What Changes

- A signal first wakes a watchdog thread that ends the process a second
  later, then restores the terminal as before. A reset that cannot be written
  no longer keeps imp alive.
- SIGHUP is handled like SIGTERM.

## Capabilities

### Modified Capabilities

- `tui`: signals end the process even when output is stuck.

## Impact

- `src/tui.rs`.
