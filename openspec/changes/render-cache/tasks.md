# Tasks

- [x] 1.1 Cache rendered rows per entry and reuse them in `frame`. Verify: existing frame tests pass, a test that an unchanged entry is not rendered again and a changed one is, and a timing of 400 turns.
- [ ] 1.2 Run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
