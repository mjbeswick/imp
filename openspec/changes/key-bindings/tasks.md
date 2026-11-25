# Tasks: key bindings for slash commands

## 1. Config

- [ ] 1.1 Test: `bind.ctrl-x-n = /model` is read into a binding with chord `ctrl+x n` and command `/model`.
- [ ] 1.2 Test: a `bind.` line naming a reserved key is ignored with a note, and other lines still load.
- [ ] 1.3 Implement: binding parse and save in `src/config.rs`, and the reserved-key list.

## 2. Commands

- [ ] 2.1 Test: `/bind ctrl+x n /model` saves the binding and shows a confirmation.
- [ ] 2.2 Test: `/bind ctrl+c /model` is refused and saves nothing.
- [ ] 2.3 Test: `/unbind ctrl+x n` removes the binding.
- [ ] 2.4 Test: `/keys reset` removes every `bind.*` line and leaves other settings.
- [ ] 2.5 Implement: the `bind`, `unbind` and `keys reset` arms in `run_command` in `src/tui.rs`.

## 3. Keys

- [ ] 3.1 Test: a bound single key runs its command when pressed in the composer.
- [ ] 3.2 Test: a bound chord runs its command when both keys arrive within 1.5 seconds.
- [ ] 3.3 Test: after the 1.5 second wait, the first key does what it did before.
- [ ] 3.4 Test: during capture, Enter saves the captured chord and Esc cancels it.
- [ ] 3.5 Implement: the capture state, the chord timer, and the lookup that runs a bound command.

## 4. Display and docs

- [ ] 4.1 Test: the `/keys` card lists a user binding after the built-in keys.
- [ ] 4.2 Implement: the list in `src/help.rs`, and the `/bind`, `/unbind` and `/keys reset` lines in `src/help.rs`.
- [ ] 4.3 Update `README.md` with the commands and the chord syntax.

## 5. Validation

- [ ] 5.1 Run `cargo test` and `openspec validate --all --strict`, and record the outcome.
