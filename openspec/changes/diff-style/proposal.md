# Proposal

## Why

Diff rows are tinted only from the sign to the end of the text, so the bands
are ragged, and a one-word edit has to be spotted by eye. Code in a diff has
no styling.

## What Changes

- Added and removed rows are tinted from the sign to the screen edge; the
  indent stays clear.
- A removed line paired with the added line that replaces it marks the words
  that differ with a stronger tint and bold.
- Rows are syntax-styled by the file's extension: keywords bold, strings in
  the success colour, numbers in the accent colour, comments dim and italic.
  Rust, shell, Python, JS/TS, and TOML/YAML comments to start; other
  extensions stay plain.
- Changed-word emphasis is laid over the syntax colours.
- Tokens are matched per line, so a string or comment spanning lines is
  styled on its first line only.

## Keyboard and resources

No keys. One word diff per line pair and one token pass per line, within the
existing 200-line cap. kmscon draws bold but likely not italic, so comments
rely on the dim colour there.

## Capabilities

### Modified Capabilities

- `tui`: diff bands, changed words, and syntax styling.
