# Proposal

## Why

The only moving indicator sits on the status line, beside the model name, far
from the work it describes. The running tool row carries a fixed frame, so a
long command appears frozen. The action is what is happening, so the spinner
belongs there.

## What Changes

- The running tool row's leading glyph animates at ten frames a second and is
  replaced by `✓` or `✗` when the call finishes.
- The status line loses the spinner and starts with the model name.
- The spinner timer runs only while a tool is running, not for the whole turn.
- Line mode is unchanged: it has no animated row and keeps its start/finish
  rows.

## Capabilities

### Modified Capabilities

- `tui`: the tool-activity requirement names the animated row glyph, and a new
  requirement keeps the status line free of spinners.

## Impact

- `src/tui.rs`: running flag on the tool row, spinner tick, status line, tests.
- `openspec/specs/tui/spec.md`.
