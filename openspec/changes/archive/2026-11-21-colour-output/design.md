# Design

## Decisions

- **SGR only.** `ESC [ … m` is kept; every other CSI, OSC, DCS, APC, and
  control byte is dropped, so a tool can colour text but never move the
  cursor or break the layout.
- **A `raw` span style** carries text with its SGR codes. Width counts only
  visible characters, the cut keeps codes whole, and each row ends with a
  reset. The renderer never wraps raw rows; they are laid out already.
- **Raw rows reuse the markdown path.** A raw entry, a shown output, or a
  rendered block becomes rows of raw spans, so the render cache covers them.
- **The model gets text.** `show` hands the model the cleaned text; when more
  than a third of its visible characters are block or braille characters,
  only a placeholder with the picture's size.
- **Renderers are set once at start,** like `pass_env`, and called from the
  markdown renderer for closed fences only, so a streaming block is not run
  half-written. Results, failures included, are cached by command, width, and
  text, so redraws never start the process again.
- **Two-second limit** on a renderer, then the block shows as code.

## Keyboard

No change.

## Resources

One short process per new diagram, then a cached string. Pictures cost their
text size in memory while shown.
