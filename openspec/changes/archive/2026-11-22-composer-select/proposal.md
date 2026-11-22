# Proposal

## Why

The draft cannot be selected. Shift+arrows move like plain arrows, so a
word cannot be marked and replaced, and the Delete key does nothing.

## What Changes

- Shift+←/→ extend a selection by a character, Shift+↑/↓ by a row, and
  Shift+Home/End to the start or end of the draft. A move without Shift
  clears it.
- The selection is drawn reversed, like the caret.
- Backspace or Delete removes the selection. Typing, a newline, or Ctrl+V
  replaces it.
- Delete removes the character after the caret. Shift+Delete does the same.
- Ctrl+C keeps its meaning.

## Keyboard and resources

New keys: Shift with the arrows, Home, End, and Delete, and Delete alone.
One more byte offset per session.

## Capabilities

### Modified Capabilities

- `tui`: a selectable draft and the Delete key.
