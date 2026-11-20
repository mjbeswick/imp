# Tasks

- [x] 1.1 Add `since` per session, the queue, Ctrl+O and `/next`, and remove the inbox view. Verify: tests for oldest first, dismiss by moving on, approval kept and moved back, empty queue.
- [ ] 1.2 Add `auto_next` (default on) and switch after a hand-off with a one-line note. Verify: tests for a switch after sending, none when off, none while typing.
- [ ] 1.3 Update `/help`, line mode, and `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
