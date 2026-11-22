# Design

## Approaches

- **Code: tint background, no bold (chosen).** Separates code from bold text
  in every theme. `theme::tint` already makes readable diff bands, and the
  text colour goes on it as in diffs: the code colour on its own tint fell
  under 4.5:1 in `default-light`, both solarized themes, and
  `catppuccin-latte`. In `mono` the tint is gone, so code falls back to
  reverse video.
- **Code: bold plus tint.** Louder, and bold code next to bold prose still
  collides. Rejected.
- **Fences: a `│ ` gutter (chosen).** Works without colour, needs no padding
  to the screen edge, and leaves copy and wrap untouched apart from two
  columns.
- **Fences: a full background.** Needs every row padded to the width and
  fights the copy label column. Rejected.
- **Mono keeps attributes (chosen).** `mono` means no colour, not no style.
  `NO_COLOR` keeps meaning no escapes, as the spec and pipes require.

## Painting

`Palette.allowed` already means escapes are allowed at all, and `on` is the
colour switch. `Palette::styled` adds SGR 1, 3, 4, and 7 under `allowed`,
colour stays under `on`. `select` reverses the row when colour is off.

`Style` gains `heading: u8` and `marker` for list markers; underline for
links comes from `Span::link`. The renderer sets them and the painter maps
them. Quote rows get `italic` with `dim`. `wrap` gives a space inside one run
that run's style, so an underline does not break between words.

`lines` draws a rule as `─` repeated to its width in `border`, before
`block` sees the line. Fence rows wrap at two columns less and lead with a
`│ ` span marked `border` and `fence`, so the copy label still finds one
block.

## Help

`help::commands` takes `color`. `/help` pushes its text behind `RAW`, so the
SGR survives and `raw_rows` drops colour when the palette is off.

## Size

Expected: 1 to 2 KB.
