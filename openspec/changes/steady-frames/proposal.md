# Proposal

## Why

While a permission question is open, the Hackberry's screen flickers once a
second. The countdown redraws the frame each second, and every row is erased
before it is rewritten; kmscon renders in software on the Pi Zero, so the
blank rows show.

## What Changes

- Each row is written first, then the rest of the line is erased, so no row
  is blank on screen between frames.
- Each frame is wrapped in synchronised output, so terminals that support it
  show it at once. Others ignore the sequence.
- Only rows that differ from the last frame are written. A countdown tick
  rewrites one row, not the screen.
- A frame is still drawn only when something visible changed.

## Keyboard and resources

No keys. Far fewer bytes per frame; one copy of the last frame is kept,
about one screen of text.

## Capabilities

### Modified Capabilities

- `tui`: how a row is cleared, and synchronised frames.
