# Tasks

- [ ] 1.1 Claim session ids with an exclusive sidecar create, in `create` and `fork`. Verify: a test that sessions created in the same second get different ids and files.
- [ ] 1.2 Refuse a path through a dangling symlink in the jail. Verify: a test that a write through a dangling link pointing outside is refused and nothing is created.
- [ ] 1.3 Run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
