# Tasks

- [ ] 1.1 Remove the space check. Run any leading-slash input as a command and keep the unknown suggestion. Verify: a test shows `/cd /tmp` sends `Cmd::SetRoot`.
- [ ] 1.2 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
