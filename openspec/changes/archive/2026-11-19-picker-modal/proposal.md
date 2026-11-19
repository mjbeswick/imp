# Proposal

## Why

Slash commands that take a value either guess at typed arguments or fall back to
three separate picker implementations. `/agent` has one, `/sessions` has
another, and the `ask` tool's choices are a third. Invalid values such as
`/verbosity loud` are only caught after Enter. `/cd` and `/image` require the
user to type a path from memory in a terminal with no file manager.

## What Changes

- Add one borderless-in-spirit picker component: title, filter text, scrolling
  list, selection, cancel. It draws as a centred bordered overlay over the
  transcript, the single exception to the one-column rule.
- Typing filters, Up/Down move, PgUp/PgDn page, Enter chooses, Esc cancels.
- `/agent`, `/sessions`, and `/verbosity` open the picker when given no value;
  a typed value still applies directly.
- `/cd` with no argument opens a directory browser rooted at the session:
  Enter enters a directory, Tab selects the highlighted one and closes, Left
  goes to the parent, and `..` stops at the session root.
- `/image` with no argument opens a browser showing directories and image
  files under the session root; Enter attaches the highlighted image.
- Path pickers never leave the session root or the jail; an absolute path is
  still available by typing it after the command.

## Capabilities

### New Capabilities

- `picker`: the shared overlay, its filtering and keys, and the three command
  pickers, directory browser, and image browser.

### Modified Capabilities

- `tui`: one centred overlay is allowed; every other row stays in the single
  chat column. Bare `/verbosity` opens the picker and bare `/cd` opens the
  browser.
- `images`: bare `/image` opens the browser and Enter attaches the highlighted
  image.

## Impact

- `src/tui.rs`: a `Picker` state, overlay compositing, key handling, the
  directory and image browsers, and the command wiring.
- `openspec/specs/tui/spec.md` and `openspec/specs/images/spec.md`: the
  one-column exception and the bare command forms.
- `src/help.rs` and `README.md`: no-argument forms and picker keys.
- Depends on `cwd-commands` and `image-paste` for the commands it enhances.
