# Proposal: command palette

## Why

Commands are typed after a slash, so a user has to know a command's name before finding it. The
slash picker already lists every command with its description, but it opens only after `/` is typed
in the composer. A key that opens the same list from anywhere makes the commands discoverable without
removing the typed form.

## What Changes

- Ctrl+P opens the command palette: the list of commands from the same table as `/help`, each with its
  description, with the filter empty.
- Typing filters the list, Up and Down move the highlight, and Enter runs the highlighted command, as
  choosing it from the slash picker does.
- Esc closes the palette and leaves the composer unchanged.
- Slash commands keep working exactly as before.

## Out of Scope

- Removing, renaming, or changing the behaviour of any slash command.
- Ranking the palette by recent use.
- A key binding editor.

## Impact

- `src/tui.rs`: the Ctrl+P key opens the command picker without a slash prefix.
- `src/tui/tests.rs`: tests for opening, filtering, running, and closing the palette.
- `src/help.rs` and `README.md`: the key is listed in the keys help and the README.
