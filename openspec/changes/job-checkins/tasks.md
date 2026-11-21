# Tasks

- [x] 1.1 Add `delay_s` to background jobs and progress to the job status. Verify: tests that a delayed job waits, can be killed before it starts, and reports size and last line.
- [ ] 1.2 Add `job check`, `notify`, and the session's due notes with limits. Verify: tests for a due check-in, a notified exit, the limit of four, the 30-second floor, and line mode refusing.
- [ ] 1.3 Wake the worker for due notes and show the check-in in the chat and the queue. Verify: a test that a due check-in runs a turn and marks an out-of-view session replied.
- [ ] 1.4 Update the prompt and `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
