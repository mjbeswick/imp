# Design

## Approaches

- **Reuse the composer's editing (chosen).** The answer row borrows the
  same edit function over its own text and caret.
- **Copy the keys into the answer handler.** Two edit paths that drift.

## Details

Extract the composer's text edits into a function over `(text, caret)` and
call it from both. The answer row gains a caret field.
