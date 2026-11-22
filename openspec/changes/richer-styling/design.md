# Design

## Approaches

- **Code: tint background, no bold (chosen).** Separates code from bold text
  in every theme. `theme::tint` already makes readable diff bands. In `mono`
  the tint is gone, so code falls back to reverse video.
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

`Palette` gains `attrs`: true when escapes are allowed at all. `on` stays the
colour switch. `paint_spans` adds SGR 1, 3, 4, and 7 under `attrs`, colour
under `on`. `select` reverses the row when `attrs` is set and colour is off.

`Style` gains `heading: u8`, `link` styling comes from `Span::link`, and
`marker` for list markers. The renderer sets them; the painter maps them.
Quote rows get `italic` with `dim`. The rule is drawn by the painter at the
row's width from a `rule` marker, so it never wraps.

Rule: `block` returns a single `─` with `border`; `lines` repeats it to the
width.

## Help

`help::commands` takes `color`. `/help` pushes its text behind `RAW`, so the
SGR survives and `raw_rows` drops colour when the palette is off.

## Size

Expected: 1 to 2 KB.
