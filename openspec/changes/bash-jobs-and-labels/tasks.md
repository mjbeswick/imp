# Tasks

- [ ] 1.1 Capture `bash` output through a log file and hand a timed-out model call to the job table. Verify: a test adopts a command past a short timeout and `job wait` returns its exit code; a typed command is still terminated.
- [ ] 1.2 Add the optional `description` argument and show it as the row summary; name the command in a labelled call's confirmation. Verify: `describe` tests and a confirmation frame test.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
