# Tasks

- [ ] 1.1 Toggle the session mode and the path jail together. Verify: a strict session toggles to yolo with the jail off, and back to strict with the jail on.
- [ ] 1.2 Wire `/yolo` in chat and line mode, with one transcript line per switch. Verify: both front-ends print the same sentence.
- [ ] 1.3 Add `/yolo` to the command table. Verify: `/help` lists it and the command-count test passes.
- [ ] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
