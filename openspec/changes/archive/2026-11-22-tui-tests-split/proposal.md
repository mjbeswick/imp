# Proposal

## Why

`src/tui.rs` is 12,772 lines: about 7,400 of code and 5,400 of tests. Finding
code means scrolling past the tests.

## What Changes

- The tests move to `src/tui/tests.rs` as `#[cfg(test)] mod tests;`.
- No code or behaviour changes. The same tests pass.

## Keyboard and resources

None.
