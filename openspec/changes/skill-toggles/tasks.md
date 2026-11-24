# Tasks: skill toggles

## 1. Loader and config

- [x] 1.1 Test: `load_skills` omits a skill whose `skill.<name>.enabled` is false, and keeps it when the flag is absent or true. Confirm it fails first.
- [x] 1.2 Implement: read the flag and filter in `load_skills`.
- [x] 1.3 Test: the config round-trips `skill.<name>.enabled`.

## 2. Session behaviour

- [x] 2.1 Test: a disabled skill is absent from the prompt index and `skill_load` refuses it.
- [x] 2.2 Implement: `skill_load` returns the disabled message for a disabled skill.

## 3. Picker

- [ ] 3.1 Test: `/skills` lists each skill with its source and state, and space toggles the selected one.
- [ ] 3.2 Implement: the `/skills` picker writes the flag and reloads.
- [ ] 3.3 Add `/skills` to the help command list.

## 4. Validation

- [ ] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Record the measured size.
- [ ] 4.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
