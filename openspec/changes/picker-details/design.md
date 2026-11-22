# Design

## Approaches

- **Wrap the highlighted row (chosen).** One extra line, read at the cursor.
  The list keeps its density and only the row you are on grows.
- **Two lines for every row.** Uniform and never cut, but halves the visible
  items. The slash list holds 23 commands and the box shows 8. Rejected.
- **A marquee or horizontal scroll.** Animates, and cannot be read at a
  glance. Rejected.
- **A detail line in the footer.** Full width, but it pulls your eye off the
  cursor and costs the key hints. Rejected.

## Layout

`picker_row` returns the lines for one item: the marker, the padded label, and
the first part of the description, then continuation lines indented to the
description column. The description wraps at spaces to the inner width, at
most two lines, and is cut with `…` when it still does not fit. Only the
highlighted row asks for more than one line; every other row is cut exactly
as today.

The overlay builds the visible lines first, then sizes the box from them. If
that height would not leave the screen, it rebuilds without wrapping, so the
single-line cut is the fallback rather than a lost row. Click targets follow
the drawn lines, so a click on a continuation line picks its item.

Column sizing is unchanged: the right column stays whole and the label gives
way, except in the command picker, where the name stays whole. The only new
behaviour is that a cut description on the highlighted row is shown in full
on the next line.

## Size

Expected: under 1 KB.
