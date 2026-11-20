# Tasks

- [x] 1.1 Search session text on Enter when no title matches, with snippets. Verify: tests for a hit, no hit, case, and resume from a hit.
- [x] 1.2 Show a spinner on working live sessions in `/sessions`. Verify: tests for a working row, an idle row, and the repaint while open.
- [x] 1.3 End background jobs with their session. Verify: a test that dropping `Jobs` stops a running job.
- [ ] 1.4 Update `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
