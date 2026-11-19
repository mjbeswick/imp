# Design

## Approaches

1. **Fix the prefix rules and drop the width cut.** Chosen: the renderer already
   wraps by word and hard-breaks long tokens; only the entry prefix and the
   summary builder fight it.
2. **A table layout.** Too much machinery for one glyph and a hanging indent.
3. **Keep the 120-character summary.** Saves rows but hides the command and its
   duration, which is what the user is reading.

## Decisions

- In `frame`, `Role::Tool` and `Role::Error` share a first-line prefix: none.
  The mark is already the first character of a tool row. Continuations keep the
  existing two-space prefix. Plain errors keep no prefix; their text wraps at
  two spaces.
- `summarise` keeps flattening newlines into spaces but no longer cuts at 120
  characters. `describe` therefore returns the whole command, path, question,
  or query. The composer and `fit` still clamp where a hard limit is required.
- The ask row uses the same builder, so the question wraps. Options are joined
  with ` · ` and wrap with it; the interface does not add a second row per
  option.
- A tool row's duration stays in the summary text, so a wrapped row ends with
  the time.
- Tests: success and failure rows both begin with their mark at column one;
  a 300-character command produces more than one row and keeps its closing
  duration; a long failure reason wraps; an ask question wraps.

## Tests

- Frame a successful and a failed tool row and compare the leading columns.
- Frame a long command and count the wrapped rows.
- Frame a long failure preview and assert the text appears across rows.
- Frame an ask row and assert the question is wrapped, not clipped.
