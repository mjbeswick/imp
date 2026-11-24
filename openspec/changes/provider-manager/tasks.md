# Tasks: provider manager

## 1. Providers list

- [x] 1.1 Test: `/provider` lists every built-in and named provider, each with its state.
- [x] 1.2 Implement: list rows and states in the `/provider` picker.
- [x] 1.3 Test: Enter opens the config for the selected provider.
- [x] 1.4 Implement: Enter opens the provider config.

## 2. Keys

- [x] 2.1 Test: plain `u` makes the selected provider active.
- [x] 2.2 Test: space toggles enabled and saves `provider.<name>.enabled`. Judgement call: the provider list takes no type-to-filter, so its letters are keys; text typed after /provider still types into the add-prefix.
- [x] 2.3 Test: plain `d` deletes a named provider and refuses a built-in with a message.
- [x] 2.4 Test: plain `n` opens a new provider config.
- [x] 2.5 Implement: the keys above.

## 3. Config modal

- [x] 3.1 Test: Save writes the edited fields and keeps comments.
- [x] 3.2 Test: Cancel leaves the config and keys file unchanged.
- [ ] 3.3 Test: a typed key is written to the keys file and not to the config.
- [ ] 3.4 Implement: Save and Cancel, and the key routing.

## 4. Model list

- [x] 4.1 Test: the `/model` list carries no provider actions.
- [x] 4.2 Keys card and help list the new keys.

## 5. Validation

- [x] 5.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Record the size.
- [x] 5.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
