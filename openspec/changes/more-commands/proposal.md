# Proposal

## Why

The interface has no way to open a shell, choose a model, manage provider
profiles, or park a thought while a turn runs. Sessions are bound to one model
and one provider until restart, and typing while the agent works either queues
blindly or interrupts the flow.

## What Changes

- `/shell` suspends the full-screen interface and runs `$SHELL`; line mode runs
  it directly and returns.
- `/model` selects a model for the session: bare opens the picker with the
  current model, the provider default, and recent models; `/model <name>` sets
  it directly.
- `/provider` lists built-in and custom profiles and supports
  `add`, `edit`, `delete`, and `use`. Custom profiles live in a managed section
  of the config file, so comments and other keys survive.
- `/stash` keeps session-bound messages: `/stash <text>` adds, bare lists,
  `pop [n]` loads an entry into the composer, `delete [n]` removes it, and
  `pop all` loads every entry joined by blank lines.
- The command count moves from fifteen to nineteen.

## Capabilities

### Modified Capabilities

- `tui`: four new commands, each with its own requirement.

## Impact

- `src/tui.rs`: command parsing, shell suspend/restore, picker wiring, stash.
- `src/config.rs`: managed provider section and recents file.
- `src/help.rs` and `README.md`.
- `openspec/specs/tui/spec.md`.
