# Design

## Approaches

1. **One table helper for both front-ends.** Chosen: `usage` already builds
   rows by hand; a helper removes the drift and makes wrapping testable.
2. **A help crate or generated man page.** Too much for a single binary with no
   dependencies.
3. **Keep flat lists and rely on the renderer.** The renderer wraps prose, not
   aligned columns, so continuations lose their indent.

## Decisions

- One description column for the whole output: two spaces after the longest
  left cell in any section, capped at 26 columns. `row(out, left, right,
  width, column)` writes `  {left}` padded to that column, then wraps `right`
  to the remaining width. Continuations begin at the column.
- A left cell longer than the cap keeps its own row, and its description
  starts on the next row at the column. No cell is cut.
- `usage(width)` and `commands(width)` build sections from
  `&[(left, description)]` slices and measure the column across all of them
  before writing a row. Chosen over per-section widths, which made each
  section's descriptions start at a different column.
- Option syntax is uniform: `--flag <VALUE>`, `-c, --continue`, and
  `(default: X)` instead of bracketed defaults.
- `/help` groups commands by area: conversation, model and agent, workspace,
  session, interface. Keys become rows (`Enter`, `Shift+Enter`, `Esc`, ...)
  instead of one long sentence. The shell note stays last.
- `commands(width)` takes a width, defaulting to 80 in the chat and the
  terminal width in line mode. The transcript renderer still wraps anything
  longer.
- Tests: one shared column, a 60-column render with no overflow, all flags and
  all twenty commands present, and identical sections in both front-ends.

## Tests

- Build `usage(60)` and assert every row is at most 60 characters and no flag
  is cut.
- Assert every description in an output starts at the same column, and a
  left cell over the cap moves its description to the next row.
- Build `commands(60)` and check grouping and wrapping.
- Check all twenty command names and every flag remain present.
