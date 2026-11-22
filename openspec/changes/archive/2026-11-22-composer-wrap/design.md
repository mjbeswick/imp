# Design

## Approaches

- **Soft wrap with drawn-row movement (chosen).** Reads like a paragraph;
  Up and Down follow what is on screen.
- **Soft wrap, Up and Down by newline.** Simpler, but Up in a long wrapped
  line would replace the draft with history. Rejected.
- **Horizontal scroll per line.** Keeps one row per line but hides most of a
  long message. Rejected.

## Layout

One function maps the draft to drawn rows: `Vec<(start, end)>` byte ranges,
splitting each newline-separated line at the last space that fits in
`cols - 2`, or hard at `cols - 2` when none does. The caret's drawn row is
the range that holds the cursor. `composer()` draws from it and the existing
cap and scroll apply to drawn rows. Up and Down use the same map, keeping the
column within the row, and fall through to history only at the first or last
drawn row.

## Size

Expected: under 1 KB.
