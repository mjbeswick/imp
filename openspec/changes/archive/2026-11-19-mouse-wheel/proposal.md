# Proposal

## Why

In iTerm the mouse wheel scrolls the terminal's own scrollback, showing what
was on screen before imp started, instead of scrolling the conversation. imp
never asks for mouse events, so the terminal handles the wheel itself.

## What Changes

- The chat turns on SGR mouse reporting with the alternate screen, and turns it
  off on exit, on a crash, and while `/shell` or an attached command runs.
- The wheel scrolls the transcript three rows at a time; scrolling down to the
  end resumes following. In an open picker it moves the highlight.
- Clicks are ignored. Text is selected with the terminal's modifier, Option in
  iTerm.
- A `mouse=on|off` setting, default `on`; `off` leaves the mouse to the
  terminal as today.

## Capabilities

### Modified Capabilities

- `tui`: mouse wheel scrolling and the `mouse` setting.

## Impact

- `src/tui.rs`, `src/config.rs`, `src/init.rs`, `README.md`.
