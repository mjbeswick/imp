# Proposal

## Why

The inbox is a list to read and manage. What the user wants is the next thing
that needs them: the oldest unanswered reply or approval, then the one after,
without a view in between.

## What Changes

- Ctrl+O and `/next` go to the session that has waited longest: an approval,
  a question, or a reply finished out of view.
- Moving on from a reply counts it as read. An approval is never answered by
  moving on; it goes to the back of the queue.
- `auto_next` (on by default) moves to the next waiting session after the
  user sends a message or answers a prompt, with a one-line note saying where.
- The inbox view, its cards, number keys, and `x` close are removed.
  `/next` takes `/inbox`'s place, so the command count stays at 21.
- The status line's `N waiting` is the queue length.

## Capabilities

### Modified Capabilities

- `inbox`: the queue replaces the list.
- `tui`: `/next` replaces `/inbox`; the keys card loses the inbox hint.

## Impact

- `src/tui.rs`, `src/config.rs`, `src/help.rs`, `src/main.rs`, `README.md`.
