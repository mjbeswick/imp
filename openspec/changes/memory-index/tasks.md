# Tasks

- [x] 1.1 Add `Memory::index(limit)` and list it in the prompt, sorted, capped at 40. Verify: a prompt test with a note shows its slug and title, and a test with 45 notes lists 40.
- [x] 1.2 Make search match every query word in any order. Verify: a store test finds a note by two words that are not adjacent.
- [x] 1.3 Update `README.md`, run the checks, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
