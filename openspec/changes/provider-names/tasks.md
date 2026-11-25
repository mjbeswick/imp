# Tasks: provider names

## 1. Name rules

- [ ] 1.1 Test: `default_provider_name` returns the dialect when it is free, and `<dialect>-2`, `<dialect>-3` when taken.
- [ ] 1.2 Test: `check_provider_name` refuses a built-in name, refuses a name held by another saved profile, accepts the profile's own name when editing, and refuses an invalid name.
- [ ] 1.3 Implement: `default_provider_name` and `check_provider_name` in `src/config.rs`.

## 2. Form

- [ ] 2.1 Test: the form opened for a new custom provider has a `name` field pre-filled with the default name.
- [ ] 2.2 Test: configuring the built-in `deepseek` opens a form whose name is `deepseek`, not `deepseek-custom`.
- [ ] 2.3 Implement: a `name` field first in the form, and the default name for new and copied profiles in `src/tui.rs`.

## 3. Save and rename

- [ ] 3.1 Test: saving a name that is a built-in or another profile's name shows an error and keeps the form open.
- [ ] 3.2 Test: saving a profile with its own name succeeds, and a rename saves the profile under the new name and removes the old one.
- [ ] 3.3 Test: renaming the active profile keeps `cfg.active_name` pointing at the renamed profile.
- [ ] 3.4 Implement: the checks and rename in `finish_provider_form` in `src/tui.rs`.

## 4. Docs and validation

- [ ] 4.1 Update `README.md` line 166, which names the `deepseek-custom` copy.
- [ ] 4.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Known pre-existing failures in `tests/tty.rs` are noted, not fixed.
