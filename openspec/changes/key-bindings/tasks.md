# Tasks: key bindings for slash commands

## 1. Config

- [x] 1.1 Test: `bind.ctrl-x-n = /model` is read into a binding with chord `ctrl-x-n` and command `/model`. Covered by `a_bind_line_reads_as_a_chord_and_a_command`.
- [ ] 1.2 Test: a `bind.` line naming a reserved key is ignored with a note, and other lines still load. Partly done: the line is ignored and the rest loads (`a_bind_line_on_a_reserved_key_is_ignored_and_the_others_load`). The note is not shown yet.
- [x] 1.3 Implement: `parse_chord`, the reserved-key list and the `bind.` arm in `src/config.rs`. Saving bindings is not yet written.

## 2. Commands

- [x] 2.1 Test: `/bind ctrl+x n /model` saves the binding and shows a confirmation. Covered by `bind_saves_a_chord_to_the_config_and_the_session`. Bare `/bind` (capture) is task 3.
- [x] 2.2 Test: `/bind ctrl+c /model` is refused and saves nothing.
- [x] 2.3 Test: `/unbind ctrl+x n` removes the binding.
- [x] 2.4 Test: `/keys reset` removes every `bind.*` line and leaves other settings.
- [x] 2.5 Implement: the `bind`, `unbind` and `keys reset` arms in `run_command` in `src/tui.rs`.

## 3. Keys

- [x] 3.1 Test: a bound single key runs its command when pressed in the composer. Covered for Ctrl keys by `a_bound_single_key_runs_its_command_from_the_composer`; printable keys are not bound yet.
- [ ] 3.2 Test: a bound chord runs its command when both keys arrive within 1.5 seconds.
- [ ] 3.3 Test: after the 1.5 second wait, the first key does what it did before.
- [ ] 3.4 Test: during capture, Enter saves the captured chord and Esc cancels it.
- [ ] 3.5 Implement: the capture state, the chord timer, and the lookup that runs a bound command.

## 4. Display and docs

- [ ] 4.1 Test: the `/keys` card lists a user binding after the built-in keys.
- [ ] 4.2 Implement: the list in `src/help.rs`, and the `/bind`, `/unbind` and `/keys reset` lines in `src/help.rs`.
- [ ] 4.3 Update `README.md` with the commands and the chord syntax.

## 5. Command lists

- [ ] 5.1 Test: the slash list shows `ctrl+x n` beside `/model` when that binding exists, and shows no key for an unbound command.
- [ ] 5.2 Test: the command palette shows a bound key beside its command.
- [ ] 5.3 Implement: read the bindings into the command rows in `src/tui.rs`, for the slash picker and the palette.

## 6. Validation

- [ ] 6.1 Run `cargo test` and `openspec validate --all --strict`, and record the outcome.
