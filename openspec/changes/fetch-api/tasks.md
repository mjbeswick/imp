# Tasks

- [ ] 1.1 Send any method with headers and a body, and keep chosen response headers. Verify: a test against a local server for POST with a JSON body, DELETE, and a `Location` header.
- [ ] 1.2 Add `method`, `headers`, and `body` to `fetch`, `auth` lines in the config, and approvals by method and credentials. Verify: tests for credentials on the right host only, no redirect with credentials, a write asking every time, a credentialed GET asking once, and the key redacted.
- [ ] 1.3 Update the prompt and `README.md`, run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
