# Design

## Approaches

1. **Lay tables out in the markdown renderer.** Chosen: it already turns a
   reply into width-bounded rows of spans, so a table is one more block kind
   and wrapping, styling, and links come for free.
2. **Pass tables through as fenced text.** Rejected: alignment breaks as soon
   as the table is wider than the screen.
3. **A table crate.** Rejected: no runtime dependencies, and the layout is a
   few dozen lines.

## Decisions

- A table is a row containing `|` followed by a separator row of dashes with
  optional colons, then every following row that contains `|`. Leading and
  trailing pipes are optional; `\|` is a literal pipe. Rows are padded or cut
  to the header's cell count.
- Natural column width is the widest cell's plain text. The frame costs
  `3 × columns + 1`. While the total is too wide, the widest column loses one
  column, down to 3. If that still does not fit, the table becomes records:
  for each row, one line per column, `header: value`, with a blank line
  between rows.
- Cells wrap with the existing word wrap at their column width; a row is as
  tall as its tallest cell. Alignment pads left, right, or both.
- `Style` gains `border`, painted in `Role::Border`. Header cells are bold.
- `render::clean` removes CSI sequences (`ESC [` to a final byte), OSC
  sequences (`ESC ]` to BEL or `ESC \`), any other `ESC` pair, and every other
  control character except newline and tab. The chat applies it to replies,
  thinking, and tool text and turns tabs into four spaces; line mode applies it
  to everything it prints.
- The prompt line about width becomes: reply in markdown; tables render as
  boxed tables, so keep them inside the width and prefer lists when narrow.

## Tests

- A three-column table renders with box lines, bold headers, and alignment.
- A wide table shrinks and wraps, and every row fits the width.
- A table that cannot fit becomes `header: value` records.
- Escaped pipes, short rows, and a row without trailing pipe.
- `clean` drops CSI, OSC 52, BEL, and carriage returns, and keeps text.
- A reply with an escape draws no escape of its own in the frame.
