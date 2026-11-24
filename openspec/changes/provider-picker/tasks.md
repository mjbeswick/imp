# Tasks: provider picker

## 1. Reproduce

- [ ] 1.1 Test: in the provider picker, select `custom` and press Enter. Expected: the pending state is `ProviderForm` and the active provider is unchanged. Confirm the test fails before the fix.
- [ ] 1.2 Find the path that reaches `use_provider` on Enter, and record it in the design.

## 2. Stable rows

- [ ] 2.1 Test: with `wrap` on, an unselected provider row with a long description draws two lines, the same as the selected row. Other picker kinds still wrap only the selected row.
- [ ] 2.2 Test: the total line count of the provider list is the same for every selection.
- [ ] 2.3 Implement: `picker_lines` wraps provider rows regardless of selection.

## 3. Enter opens the editor

- [ ] 3.1 Implement: the provider Enter branch opens the editor for every row. Onboarding keeps its own branch.
- [ ] 3.2 Test: `u` on a provider row still switches the active provider.

## 4. Validation

- [ ] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [ ] 4.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
