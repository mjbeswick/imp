# Proposal

## Why

Bare `/cd` starts at the session root and refuses to climb above it, so a
sibling or parent directory can only be reached by typing the path. After a
command word and a space the overlay closes, even when the command has a short
list of valid values, so those names have to be remembered. The picker filter
also has no visible cursor, which makes typing into it feel disconnected from
the line shown in the box.

## What Changes

- The directory browser keeps the session root as its starting point but can
  climb above it with Left, up to the filesystem root.
- A valid command word followed by a space opens the value picker for commands
  with enumerable values: `/agent`, `/model`, `/theme`, `/verbosity`,
  `/provider`, `/cd`, and `/image`.
- The picker filter takes the keyboard and draws a visible cursor in the filter
  line; typed characters append to the command line after the prefix.
- Commands with free text (`/stash`, `/rename`, and a typed `/cd` or `/image`
  path) keep the composer.

## Capabilities

### Modified Capabilities

- `picker`: directory navigation above the root, and filter focus with a
  cursor.
- `tui`: slash autocomplete covers command arguments as well as names.

## Impact

- `src/tui.rs`: browser bounds, argument detection, picker prefix and cursor.
- `openspec/specs/picker/spec.md` and `openspec/specs/tui/spec.md`.
- `README.md`.
