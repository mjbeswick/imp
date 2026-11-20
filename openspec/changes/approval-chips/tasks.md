# Tasks

- [ ] 1.1 Add `allow` lines to the config, shared across sessions, appended to the config file on `a`; rename the session answer to `s`. Verify: agent tests for `s` per session, `a` across a new session and in the file, strict still asking, deny first.
- [ ] 1.2 Draw key chips on the confirmation row and accept `s` and `a` in the TUI, the inbox, and line mode. Verify: frame tests for the chip text and the countdown at the right edge, an inbox `s` test, line-mode parse tests.
- [ ] 1.3 Update `README.md` and `/help`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
