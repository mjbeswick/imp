# Proposal

## Why

The composer moves and deletes one character at a time. The keys people
already use in shells, Option or Alt with a key for words and the readline
Ctrl keys for lines, do nothing.

## What Changes

- Option+←/→ (or Alt+B/F) move by word.
- Option+Backspace and Ctrl+W delete the word before the caret; Option+Delete
  and Alt+D the word after it.
- Shift+Option+←/→ select by word, so typing replaces the word.
- Ctrl+A goes to the start of the row, Ctrl+U deletes to it, Ctrl+K deletes
  to the end of the row.
- The keys card lists them.

## Keyboard and resources

Ctrl+E keeps expanding tool rows; End covers the end of the row. macOS
Terminal sends Option+key as Alt only with "Use Option as Meta key" on;
imp reads both the Alt bytes and the modified-arrow sequences.

## Capabilities

### Modified Capabilities

- `tui`: word and line editing keys.
