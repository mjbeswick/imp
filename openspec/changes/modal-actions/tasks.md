# Tasks

- [ ] 1.1 Add row actions and the footer. Declare shortcuts per row, render them, and run them only when the filter is empty. Verify: `r` renames a highlighted session, and `r` filters once text is typed.
- [ ] 1.2 Move session shortcuts into row actions. Verify: `n`, `r`, and `d` still work and appear in the footer.
- [ ] 1.3 Use the picker for slash autocomplete. Seed the filter, keep prefix ranking, and wire Tab, Enter, and Esc. Verify: `/co` narrows, Tab completes, Enter runs.
- [ ] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
