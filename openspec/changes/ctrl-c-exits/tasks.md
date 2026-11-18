# Tasks

- [x] 1.1 Make the second Ctrl+C during a turn exit, and clear the latch when the turn ends. Verify: a streaming app cancels on the first press, quits on the second, and a finished turn resets the latch.
- [x] 1.2 Show `cancelling · ctrl+c again to exit` on the first press. Verify: the frame after that press contains the hint.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
