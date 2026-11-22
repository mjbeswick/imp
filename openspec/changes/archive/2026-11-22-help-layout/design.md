# Design

## Approaches

- **Adjust the shared table (chosen).** One renderer serves `--help`,
  `/help`, and line mode, so every surface gets the fixes.
- **A separate narrow layout.** Two renderers to keep in step. Rejected.

## Table

`table()` takes `color`. A cell starting with `--` is drawn with four extra
spaces. A description is split at ` (default: `; the default is drawn on the
next row at the description column. `EXAMPLES` rows draw `# desc` then the
command, and do not count toward the column width. When fewer than
`MIN_TEXT` (24) columns remain, the column becomes six and every
description goes under its cell.

`--help` colours when stdout is a terminal and `NO_COLOR` is unset.

## Size

Expected: under 1 KB.
