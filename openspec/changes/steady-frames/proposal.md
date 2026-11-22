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
- Deferred: sending only the rows that changed. Add it if the flicker or the
  cost of a full frame on the Pi still shows after this change.

## Keyboard and resources

No keys. Fewer bytes per frame; no extra work.

## Capabilities

### Modified Capabilities

- `tui`: how a row is cleared, and synchronised frames.
