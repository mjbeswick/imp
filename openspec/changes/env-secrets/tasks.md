# Tasks

- [x] 1.1 Strip secret variables from the agent's commands and jobs, with `pass_env`. Verify: tests that a stripped variable is empty in a command and a passed one is set.
- [ ] 1.2 Redact secret values from tool results and typed output, and ask when a command names a stripped variable. Verify: tests for redaction and for `echo $NAME` asking.
- [ ] 1.3 Update `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
