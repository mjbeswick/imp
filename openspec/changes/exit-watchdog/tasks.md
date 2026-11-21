# Tasks

- [x] 1.1 Wake a watchdog from the signal handler and handle SIGHUP. Verify: a pty check that a process whose terminal stopped reading exits within two seconds of SIGTERM.
- [ ] 1.2 Run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
