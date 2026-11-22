# Proposal

## Why

Ctrl+P is the palette key people reach for, but it currently steps back
through live sessions. The command list only opens from `/`, and the keys live
on a separate card, so "what can I do" takes two lists and a memory.

## What Changes

- `Ctrl+P` opens a palette: the slash commands and the useful keyboard
  actions in one filterable list, each with its shortcut in the description.
- Enter runs the highlighted command, or replays the key action.
- Esc closes; typing filters; the count of slash commands stays at 25.
- `Ctrl+N` keeps cycling live sessions forward. `Ctrl+P` no longer goes back,
  because the palette is the common meaning.
- The keys card lists both keys. `?` and `/` stay as they are.

## Keyboard and resources

`Ctrl+P`, shown in the keys card. One extra picker kind; no new process.

## Capabilities

### Modified Capabilities

- `tui`: a palette on Ctrl+P, and session cycling on Ctrl+N alone.
