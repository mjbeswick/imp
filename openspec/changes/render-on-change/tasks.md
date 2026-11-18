# Tasks

- [ ] 1.1 Render only on change. Verify: an idle session writes no bytes, measured with `/proc/<pid>/io` on the Pi.
- [ ] 1.2 Throttle the streaming spinner to ten frames a second. Verify: a streaming turn redraws no faster than 100 ms apart.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
