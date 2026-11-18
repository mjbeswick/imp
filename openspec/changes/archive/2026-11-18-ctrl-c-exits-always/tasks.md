# Tasks

- [x] 1.1 Exit on Ctrl+C from every state. Verify: tests quit from idle, a streaming turn, an open confirmation, and an open picker.
- [x] 1.2 Remove the cancel-then-exit latch. Verify: no `cancel_requested` remains and no test references the two-press hint.
- [x] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
