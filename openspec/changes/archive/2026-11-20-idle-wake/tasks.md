# Tasks

- [x] 1.1 Pick the loop wait from what is active and wake on SIGWINCH. Verify: a test of the wait for streaming, a recent key, a prompt, and idle; a resize redraws at once in a pty.
- [x] 1.2 Remove `Theme.light` and run `ctrl_c_exits_at_once_when_idle`. Verify: no dead-code warning for `light`, and the test runs and passes.
- [x] 1.3 Run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
