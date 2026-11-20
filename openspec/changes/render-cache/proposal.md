# Proposal

## Why

Every redraw renders the whole transcript again: 0.44 ms at 10 turns, 3.9 ms
at 100, and 15.8 ms at 400 on a Mac. The Pi Zero is 10 to 20 times slower, so
long sessions stutter while a reply streams.

## What Changes

- Each entry's rendered rows are kept and reused until its text, the width,
  or the markdown setting changes.
- Only the entry being streamed, or a running tool row, renders again.

## Capabilities

### Modified Capabilities

- `tui`: redraws reuse rendered rows.

## Impact

- `src/tui.rs`.
