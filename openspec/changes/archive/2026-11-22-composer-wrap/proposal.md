# Proposal

## Why

A long single-line message does not wrap: the composer shows its tail behind
a leading ellipsis, so most of it is hidden on the Hackberry's 54 columns.

## What Changes

- Long lines wrap at word boundaries; a word wider than the screen breaks
  mid-word. Wrapped rows start with two spaces.
- The composer still grows to at most eight rows or half the screen, then
  scrolls to keep the caret row visible.
- Up and Down move by screen row within wrapped text. Only Up on the top row
  recalls history, and Down on the bottom row moves forward through it.

## Keyboard and resources

No new keys. Wrapping runs on the draft only, once per frame.

## Capabilities

### Modified Capabilities

- `tui`: composer wrapping and row movement.
