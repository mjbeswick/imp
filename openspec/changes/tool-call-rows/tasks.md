# Tasks

- [x] 1.1 Summarise a call by tool from its parsed arguments. Verify: tests cover `bash`, `write`, `edit`, `read`, `search`, `memory`, `ask`, and `skill_load` shapes.
- [x] 1.2 Draw one row per call and update it in place on finish. Verify: a frame test after start and finish shows one row per call with `✓` and the duration.
- [x] 1.3 Time only the tool, not the confirmation wait. Verify: a test with a slow confirmation reports milliseconds for a fast command.
- [x] 1.4 Report the tool's own success flag. Verify: a refused write renders as an error with its reason.
- [x] 1.5 Add Ctrl+O to expand the last call. Verify: a key test pushes the full arguments and output, and a second press collapses it.
- [ ] 1.6 Allow writes under `~/.config/imp` while keeping the jail elsewhere. Verify: a tools test writes a skill file in `normal` mode and still refuses the main config.
- [ ] 1.7 Tell the model that `/reload` re-reads skills. Verify: the baseline prompt mentions `reload` and a test asserts it.
- [ ] 1.8 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
