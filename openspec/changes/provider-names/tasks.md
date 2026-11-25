# Tasks: provider names

## 1. Name rules

- [x] 1.1 Test: `default_provider_name` returns the dialect when it is free, and `<dialect>-2`, `<dialect>-3` when taken.
- [x] 1.2 Test: `check_provider_name` refuses a built-in name, refuses a name held by another saved profile, accepts the profile's own name when editing, and refuses an invalid name.
- [x] 1.3 Implement: `default_provider_name` and `check_provider_name` in `src/config.rs`.

## 2. Form

- [x] 2.1 Test: the form opened for a built-in copy has a `name` field pre-filled with the default name, `openai-2`.
- [x] 2.2 Test: configuring the built-in `deepseek` opens a form whose name is `deepseek-2`, not `deepseek-custom`.
- [x] 2.3 Implement: a `name` field as the last form field, and the default name for new and copied profiles in `src/tui.rs`. The name is last so the existing field indices keep their meaning.

## 3. Save and rename

- [x] 3.1 Test: saving a name that is a built-in name shows an error and keeps the form open.
- [x] 3.2 Test: saving a saved profile under its own name succeeds, and renaming it replaces the old name.
- [x] 3.3 Test: renaming the active profile keeps `cfg.active_name` pointing at the renamed profile.
- [x] 3.4 Implement: the checks, the rename, and the active-profile update in `finish_provider_form` in `src/tui.rs`.

## 4. Docs and validation

- [x] 4.1 Update `README.md`, which named the `deepseek-custom` copy, and describe the `name` field.
- [x] 4.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 819 unit tests pass. The two `tests/tty.rs` failures are pre-existing (they also fail on `main`) and are not fixed here.
