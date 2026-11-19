# Proposal

## Why

The agent replies in markdown, but tables are not rendered: the chat shows raw
pipes that wrap mid-row. Separately, nothing removes control sequences from
model or tool text, so an escape in a reply reaches the terminal as is and can
set the clipboard, retitle the window, or break the layout.

## What Changes

- Markdown tables in replies are drawn with box lines in the border colour,
  bold header cells, column alignment from the separator row, and inline
  markdown inside cells.
- A table wider than the screen shrinks its widest columns and wraps their
  cells; when even that cannot fit, each row is shown as a block of
  `header: value` lines.
- Control characters and escape sequences are removed from model and tool
  text before display, in the chat and in line mode. Tabs become spaces in the
  chat.
- The system prompt asks for markdown and says tables render, within the
  content width.
- Line mode still prints the markdown as written, so piped output keeps its
  tables as text.

## Out of Scope

- Colour codes from the model.
- Images.

## Capabilities

### Modified Capabilities

- `tui`: tables, and clean text in both front-ends.

## Impact

- `src/render.rs`, `src/tui.rs`, `src/main.rs`, `src/prompt.rs`, `README.md`.
