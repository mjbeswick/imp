# Design: key bindings for slash commands

## Context

- `Key` is the decoded key type (`src/tui.rs`, line 773). `Key::Ctrl(char)` covers Ctrl chords. The main key
  handler matches on `Key`, and Ctrl+X already means "delete queued message" when a message is queued
  (`src/tui.rs`, around line 3961).
- Settings are read by `Config::set` (`src/config.rs`) and written by `save_setting` (`src/config.rs`, line 925).
  Unknown keys are ignored with a note (`src/config.rs`, line 501), so an older binary still loads a newer
  config.
- `/keys` draws its card from a table in `src/help.rs`.

## Decisions

- **Storage.** Each binding is `bind.<chord> = <command>`, where the chord's keys are joined with `-`:
  `bind.ctrl-x-n = /model`. A single key is `bind.ctrl-p = /skills`. Spaces are not allowed in config keys, so
  the chord is written this way and parsed back.
- **Capture.** `/bind <chord> <command>` with both parts given saves directly. `/bind` alone opens a capture:
  the next keys are shown as they arrive, Enter saves the one shown, and Esc cancels. The shown chord is
  limited to two keys.
- **Chord timer.** A first key that starts a chord is held for up to 1.5 seconds. A second key that completes
  a bound chord runs its command. If the timer runs out, or the second key matches nothing, the first key runs
  as it did before, and the second key is handled normally.
- **Reserved keys.** The reserved set is Ctrl+C, Esc and Enter. Any other key may start a chord or be bound
  alone, including keys the main handler uses, such as Ctrl+X and Ctrl+D. A binding on one of those keys
  replaces its built-in effect while the chord is bound; `/keys reset` restores it.
  Checked when a binding is saved and at each key press. A reserved key is refused, never overwritten.
- **Reset.** `/keys reset` removes every `bind.*` line with `save_setting`-style editing of the config file,
  leaving other lines as they are.
- **Running a binding.** A bound command is passed to `run_command` as if typed, so it behaves exactly like the
  slash command.

## Risks

- A chord's first key hides that key's normal effect for up to 1.5 seconds. The recommended wait is short,
  and the timer is shown in the composer while it runs.
- Terminals send some keys the same way (Ctrl+H and Backspace). The reserved set includes both, so binding
  either is refused.
- Two bindings whose first keys match form a prefix. A chord starting with a key that is also a single-key
  binding is refused, so one key never has two meanings.

## Assumptions

- The config file is the only place bindings are stored. Editing it by hand works, and a bad line is ignored
  with a note, like other unknown settings.
