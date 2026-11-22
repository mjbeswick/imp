# Proposal

## Why

The palette hides the keys for `/keys` and `/next`. Ctrl+Q and Ctrl+V have
no command, so the palette cannot reach them. "next session" with one session
does nothing and says nothing.

## What Changes

- A command row shows its key after the description when the key does the
  same thing: `/keys` shows `?`, `/next` shows `Ctrl+O`.
- Two action rows: `steer or queue` on Ctrl+Q, `paste` on Ctrl+V.
- "next session" with one session says `one session`.

## Keyboard and resources

No new keys. Two more palette rows.

## Capabilities

### Modified Capabilities

- `tui`: palette rows and the one-session note.
