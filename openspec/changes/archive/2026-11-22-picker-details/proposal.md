# Proposal

## Why

Slash command descriptions are cut in the command picker. `/verbosity` sets
the label column at 12, so a description starts at column 16. The longest,
`/goal`, is 46 characters. At 80 columns the inner width is 62, which fits it
with nothing to spare; a narrower terminal loses the end of the sentence. A
description explains a command you are about to run, so half of one is no
help.

## What Changes

- The highlighted row's description continues on a second line, indented to
  the description column, instead of being cut.
- Every other row keeps one line and keeps the `…` cut.
- Two lines at most. A description still too long ends in `…`.
- The box grows by that one line. On a screen too short for it, the frame
  falls back to the single-line cut.

## Keyboard and resources

No new keys. One extra padded line per frame, and only for the highlighted
row.

## Capabilities

### Modified Capabilities

- `picker`: the highlighted row may use a second line for its description.
