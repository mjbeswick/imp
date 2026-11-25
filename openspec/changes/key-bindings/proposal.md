# Proposal: key bindings for slash commands

## Why

Every slash command is reached by typing its name, and the command palette (Ctrl+P) lists them. A user who
runs one command often has to type it every time. Some users want a short key or chord that runs a command
directly, and a way to undo their own bindings.

## What Changes

- `/bind` starts a capture. The next key or chord is shown, then Enter saves the binding to the named
  command and Esc cancels the capture. The command is named in the same line, as `/bind ctrl+x n /model`.
- A bound key or chord runs its slash command, as if the user had typed the command and pressed Enter.
- A chord has at most two keys. The first key starts the chord; the second must follow within 1.5 seconds.
  After that wait, the first key does what it did before.
- Bindings are saved in the config under `bind.<chord>`, one line per binding.
- `/unbind <chord>` removes one binding.
- `/keys reset` removes every user binding, so the built-in keys are back.
- Built-in keys that imp uses itself, such as Ctrl+C, Ctrl+D, Esc, Enter and Tab, cannot be bound. A chord
  that clashes with one is refused with a message and nothing is saved.
- The `/keys` card lists the user's bindings after the built-in keys.

## Out of Scope

- Binding a key to a value, a setting, or a prompt. Only slash commands are bound.
- Rebinding built-in keys. Those keys stay as they are.
- Sharing bindings between machines.

## Impact

- `src/config.rs`: `bind.<chord>` settings, read and saved with the existing config functions.
- `src/tui.rs`: the `/bind`, `/unbind` and `/keys reset` commands, the capture state, the chord timer, and the
  key lookup that runs a bound command.
- `src/help.rs` and `README.md`: the new commands and the key syntax.
