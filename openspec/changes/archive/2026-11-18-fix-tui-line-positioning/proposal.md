# Proposal

## Why

The TUI enters raw terminal mode, where line feeds do not reset the column.
Rendered rows therefore drift right, including the input prompt.

## What Changes

- Emit carriage return and line feed between rendered rows.
- Add a regression test proving frames contain no bare line feeds.

## Capabilities

### Modified Capabilities

- `tui`: every rendered row starts at column one.

## Impact

- `src/tui.rs` frame rendering and tests.
- `openspec/specs/tui/spec.md` row positioning requirement.
- No dependencies, data, or configuration changes.
