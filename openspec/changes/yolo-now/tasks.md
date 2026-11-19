# Tasks

- [ ] 1.1 Share the permission mode. Add the atomic and `code`/`from_code`, pass it to `Session::new` and `App::new`, and read it in every policy check. Verify: flipping the atomic after construction changes the next call's decision.
- [ ] 1.2 Apply `/yolo` immediately. Replace the toggle command with an exact target, update the shared mode from the App before sending, and keep the worker's config and jail in sync. Verify: a mid-turn enable suppresses the next confirmation, a second `/yolo` restores the launch mode, and queued targets settle on the last one.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
