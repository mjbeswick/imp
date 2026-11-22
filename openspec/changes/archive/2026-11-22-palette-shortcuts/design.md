# Design

## Approaches

- **Show a key only when it does the same thing (chosen).** Shift+Tab and
  Tab cycle while `/mode` and `/agent` open pickers; showing them would teach
  the wrong key.
- **Show every related key.** Misleading for the two pickers.

## Details

A `COMMAND_KEYS` table maps `keys` to `?` and `next` to `Ctrl+O`;
`palette_items` appends ` · <key>` to those descriptions. Two rows join
`PALETTE_ACTIONS` and replay their keys. The one-session note is an info line.

## Size

Expected: under 1 KB.
