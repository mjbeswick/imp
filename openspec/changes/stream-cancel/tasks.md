# Tasks

- [x] 1.1 Abort the request from curl's progress callback when the cancel flag is set, and end stalled transfers. Verify: a test against a local server that stalls, cancelled within two seconds.
- [x] 1.2 Pass the cancel flag through `Transport::post`, skip the retry, and report `cancelled`. Verify: a test that a cancelled request is sent once and the turn ends cancelled.
- [ ] 1.3 Run the checks, record the size, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
