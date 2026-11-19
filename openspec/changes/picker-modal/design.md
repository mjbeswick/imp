# Design

## Approaches

1. **One state machine, one renderer, absolute-positioned overlay.** Chosen:
   the terminal is redrawn fully every frame, so the overlay only needs to
   paint its rectangle after the normal rows; closing it leaves nothing behind.
2. **A separate full-screen mode.** Loses the transcript context and the
   one-column feel.
3. **Inline rows above the composer.** Simplest, but the chosen appearance is a
   bordered overlay.

## Decisions

- `Picker { title, filter, items, index, offset, kind }`, with
  `Item { label, value }`. `kind` is `Choose(action)`, `Dir`, or `Image`.
- Rendering happens after the normal frame. The box is centred at
  `x = (cols - width) / 2` and `y = (body_rows - height) / 2`, drawn with
  `┌─┐│└─┘`, a title/filter row, up to eight item rows, and a bottom border.
  Every frame redraws the transcript, so no erase is needed on close.
- Width is `max(40, min(cols, cols * 80 / 100))`, and never wider than the
  terminal. Height is `min(items, 8) + 4`.
- Types: the selected row is the user colour, borders are dim, unmatched text
  is plain. No colour still draws the box.
- Keys take precedence over the composer while open: printable characters
  append to the filter, Backspace removes, Up/Down move, PgUp/PgDn page, Enter
  chooses, Tab selects a directory, Left goes to the parent, Esc cancels.
  Filtering is a case-insensitive substring and resets the highlight to the
  first match.
- A directory browser keeps a `current` path that starts at the session root.
  Left at the root does nothing. Enter on a directory changes `current`; Tab
  selects it. For `/image`, Enter on a file attaches it; directories and
  image-extension files only are listed.
- Noise directories are the same list as `read`: `.git`, `.jj`, `target`,
  `node_modules`, `dist`, `build`, `.venv`.
- `Dir` and `Image` never escape the session root, even through symlinks.
- Slash commands with a value bypass the picker and keep today's behaviour.
- Dependencies change existing scenarios and are updated when this lands:
  `/verbosity` bare currently reports the level and becomes a picker;
  `/cd` bare currently prints usage and becomes a browser; `/image` bare
  currently attaches nothing and becomes a browser.
- Line mode has no overlay and keeps typed arguments only.
- Tests cover filtering, paging, empty results, Esc, the root boundary, Tab
  selection, image filtering, overlay width and height, and a closed picker
  leaving no border characters.
