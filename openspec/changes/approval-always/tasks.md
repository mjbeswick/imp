# Tasks

- [x] 1.1 Add `perm::shape(cmd) -> Option<String>`: program plus first non-flag argument, `None` for compound commands. Verify: unit tests for the scenarios.
- [ ] 1.2 Make confirmations three-way, keep an always set per session, skip prompts for matching shapes, clear it on `/new`. Verify: approval count tests in `normal`, `strict`, and after `/new`.
- [ ] 1.3 Show `y yes · n no · a always` in the TUI and line mode, hide `a` where not offered. Verify: TUI and line-mode prompt tests.
- [ ] 1.4 Update `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
