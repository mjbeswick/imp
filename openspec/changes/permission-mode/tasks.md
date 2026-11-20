# Tasks

- [x] 1.1 Replace `/yolo` with `/mode` in the chat: picker with descriptions and `current`, direct value, Tab completion, error for an unknown name, applied before the worker. Verify: TUI tests for the picker, the direct value, the error, and a mid-turn confirmation.
- [x] 1.2 Replace `/yolo` with `/mode` in line mode. Verify: a line-mode test prints the list and sets the mode.
- [ ] 1.3 Update help, README, and the yolo colour tests, then run the checks and reinstall. Verify: help lists `/mode` and not `/yolo`; `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
