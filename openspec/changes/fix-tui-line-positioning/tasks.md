# Tasks

- [x] 1.1 Add a frame test asserting it contains no bare line feeds. Verify: the test fails against the current renderer.
- [ ] 1.2 Emit CRLF for every frame row boundary. Verify: the focused test passes and `cargo test` passes.
- [ ] 1.3 Validate the change and size budget. Verify: `openspec validate --all --strict` and `./scripts/size.sh` pass.
