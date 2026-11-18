# Design

## Approaches

1. **A hand-written subset in `src/render.rs`.** Parse to styled spans, wrap
   spans, paint per span. Chosen: `imp` has one dependency and 57 KB of size
   headroom, and a markdown crate plus its tree would not fit.
2. **Add `pulldown-cmark`.** Correct and complete, but the binary budget is
   630,000 bytes and the current build is 572,576.
3. **Post-process ANSI into the text.** Would break `wrap` and `fit`, which
   count characters.

## Decisions

- Two layers: block formatting per line (fences, headings, lists, quotes,
  rules), then inline spans (bold, italic, code, links).
- Width maths stays on plain text. A `Span` carries text and style; wrapping
  splits spans into words and keeps the style, so nothing measures an escape.
- OSC 8 is emitted per span per row: `ESC ] 8 ; ; url ESC \ text ESC ] 8 ; ;
  ESC \`. A wrap can therefore never split a sequence.
- `Links` is `auto`, `always`, or `never`, default `auto`, overridden by
  `IMP_LINKS`. `auto` trusts `TERM_PROGRAM` (iTerm.app, ghostty, WezTerm),
  `KITTY_WINDOW_ID`, and `VTE_VERSION`. Without support, a link renders as
  `text (url)` in the dim colour.
- Streaming: entries render on every frame, but a paragraph whose line is still
  growing is parsed as plain text until the newline arrives, which keeps a
  half-typed `[a](ht` from being treated as a link.
- Links are stripped from the composer and from tool rows: only assistant text
  and tool *output* render as markdown.
- Out of scope: tables, images, nested lists, syntax highlighting.

## Open points

None. The subset is chosen to cover what the model actually writes.
