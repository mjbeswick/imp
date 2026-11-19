# Proposal

## Why

The help text is correct but uneven. `--help` mixes option styles and column
widths between sections, and `/help` opens with a run-on key sentence followed
by one flat command list. On a narrow terminal both overflow instead of
reflowing.

## What Changes

- One table layout for both front-ends: a left column of flags or commands, an
  aligned description column, and wrapped continuations at that indent.
- `--help` gets the same syntax in every section and width-aware wrapping.
- `/help` becomes short sections: keys as rows, commands grouped by area, and
  the shell-command note last.
- Descriptions wrap to the terminal width instead of running past it.
- The content stays the same; no command or flag is added or removed.

## Capabilities

### Modified Capabilities

- `tui`: help output has a consistent table layout in both front-ends.

## Impact

- `src/help.rs`: layout helper and section data.
- `openspec/specs/tui/spec.md`, `README.md` if the sample changes.
