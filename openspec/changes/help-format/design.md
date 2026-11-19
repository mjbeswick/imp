# Design

## Approaches

1. **One table helper for both front-ends.** Chosen: `usage` already builds
   rows by hand; a helper removes the drift and makes wrapping testable.
2. **A help crate or generated man page.** Too much for a single binary with no
   dependencies.
3. **Keep flat lists and rely on the renderer.** The renderer wraps prose, not
   aligned columns, so continuations lose their indent.

## Decisions

- `row(out, left, right, width, indent)` writes `  {left}` padded to the
  section's longest left cell plus two spaces, then wraps `right` to the
  remaining width. Continuations begin at the description column.
- `usage(width)` builds sections from `&[(flag, description)]` slices; the
  left-column width is per section, not global, so `SESSIONS` does not force
  wide gaps in `OTHER`.
- Option syntax is uniform: `--flag <VALUE>`, `-c, --continue`, and
  `(default: X)` instead of bracketed defaults.
- `/help` groups commands by area: conversation, model and agent, workspace,
  session, interface. Keys become rows (`Enter`, `Shift+Enter`, `Esc`, ...)
  instead of one long sentence. The shell note stays last.
- `commands(width)` takes a width, defaulting to 80 in the chat and the
  terminal width in line mode. The transcript renderer still wraps anything
  longer.
- Tests: section alignment, a 60-column render with no overflow, all flags and
  all twenty commands present, and identical sections in both front-ends.

## Tests

- Build `usage(60)` and assert every row is at most 60 characters and no flag
  is cut.
- Assert descriptions in one section start at one column.
- Build `commands(60)` and check grouping and wrapping.
- Check all twenty command names and every flag remain present.
