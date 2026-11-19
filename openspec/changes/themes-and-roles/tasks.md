# Tasks

- [x] 1.1 Replace the six-colour theme table with eleven RGB slots, computed 256-colour fallbacks, and the eleven themes. Verify: theme table and `to_256` tests.
- [x] 1.2 Add the `Border`, `Success`, `Code`, and `Link` roles and the `select` background, and use them for rules, frames, marks, diffs, code, links, and the picker highlight. Verify: colour frame tests for the highlight, a `✓` mark, and inline code.
- [ ] 1.3 Guess the background when no theme is saved, from `COLORFGBG` and then an OSC 11 query in the chat. Verify: parsing tests and a test that a light guess selects `default-light`.
- [ ] 1.4 Update `README.md` and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
