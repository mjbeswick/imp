# Tasks

- [x] 1.1 Apply the clear-or-exit rule in the key router. A confirmation or `ask` prompt still exits; otherwise a non-empty composer is cleared and any picker closed; an empty composer quits. Verify: draft clears, empty exits, and a streaming turn survives a draft clear.
- [x] 1.2 Update the key line in `src/help.rs` and the README to `Ctrl+C clear or quit`. Verify: `/help` and `--help` show the new wording.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
