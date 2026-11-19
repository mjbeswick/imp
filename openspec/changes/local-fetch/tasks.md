# Tasks

- [ ] 1.1 Raise the size limit to 776,000 bytes in `scripts/size.sh` and `AGENTS.md`. Verify: `./scripts/size.sh` passes.
- [ ] 1.2 Add the local `fetch` tool: GET, `http` and `https` only, status line, HTML reduced to text, spill cap. Verify: scheme, HTML reduction, and local-server tests.
- [ ] 1.3 Ask once per host in `normal`, every time in `strict`, never in `yolo`. Verify: approval count tests.
- [ ] 1.4 Remove `fetch` from `search`, update the prompt and `README.md`, and run the checks. Verify: tool-list tests, `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
