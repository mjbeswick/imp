# Design: command palette

## Context

- The slash picker is `Picker::new("commands", command_items(), PickKind::Command)` (`src/tui.rs`, around
  line 4369), with `prefix = Some("/")`. `command_items` builds one row per entry of `COMMANDS`
  (`src/tui.rs`, around line 2210; the table is `src/help.rs`, line 50).
- Enter on a `Command` row copies its value into the composer and submits it (`src/tui.rs`, around line
  6199). Tab on a row fills the composer instead. Esc closes any picker.
- Ctrl+P is not bound anywhere in `src/tui.rs` (searched). The key parser maps control bytes to
  `Key::Ctrl(char)`, so `Key::Ctrl('p')` is the key to match.

## Decisions

- **Reuse the slash picker.** The palette is `Picker::new("commands", command_items(), PickKind::Command)`
  with no prefix. Enter, Esc, Up, Down and filtering behave as they already do, so there is no second
  list to keep in step with `/help`.
- **No prefix.** The palette sets `prefix = None`, so the composer is not treated as typing a command. The
  Backspace branch that depends on the prefix does nothing when the filter is empty.
- **Entry point.** `Key::Ctrl('p')` opens the palette from the composer and from a running turn, as long as
  no other picker or prompt is open. An open prompt keeps its own keys.

## Risks

- Ctrl+P may be used by a terminal for another purpose on some platforms. The terminal decides this, not
  imp. The key is listed in `/keys` so it can be found.

## Assumptions

- Enter on a command row with no arguments opens its argument picker, as it does from the slash picker. The
  palette inherits this behaviour and adds no new paths.
