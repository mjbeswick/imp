# Tasks: provider picker

## 1. Reproduce

- [x] 1.1 Test: in the provider picker, select `custom` and press Enter. Expected: the pending state is `ProviderForm` and the active provider is unchanged. Confirm the test fails before the fix.
- [x] 1.2 Find the path that reaches `use_provider` on Enter, and record it in the design.

## 2. Stable rows

- [x] 2.1 Test: with `wrap` on, an unselected provider row with a long description draws two lines, the same as the selected row. Other picker kinds still wrap only the selected row.
- [x] 2.2 Test: the total line count of the provider list is the same for every selection.
- [x] 2.3 Implement: `picker_lines` wraps provider rows regardless of selection.

## 3. Enter opens the editor

- [x] 3.1 Implement: the provider Enter branch opens the editor for every row. Onboarding keeps its own branch.
- [x] 3.2 Test: `u` on a provider row still switches the active provider.

## 4. Validation

- [ ] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [ ] 4.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.

## 5. Model list rows name their provider

- [x] 5.1 Test: every row in the `/model` list has a `provider::model` value. Confirmed it failed on the bare rows (current, default, listed, recents).
- [x] 5.2 Implement: `model_items` tags the active provider's models with its name and leaves out the recents file, which records no provider.
- [x] 5.3 Update `model_command_sets_the_session_and_records_recents` to the provider-labelled rows; recents remain in the provider-model picker.
