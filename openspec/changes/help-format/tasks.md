# Tasks

- [x] 1.1 Add the table helper and rebuild `usage` on it. Normalise option syntax, align every section to one shared column capped at 26, and wrap to the width. Verify: a 60-column render has no overflow, every description starts in the shared column, and an over-cap entry moves its description to the next row.
- [x] 1.2 Rebuild `/help` with the same helper. Move keys into rows, group commands by area, and pass the terminal width. Verify: chat, line mode, and `--help` name the same flags and twenty commands.
- [x] 1.3 Give line mode its own key rows. Verify: line-mode `/help` lists no picker, scroll, or Tab key.
- [x] 1.4 Describe `init` and `prompt`, move `!cmd` out of the shell examples, and point unknown options to `--help`. Verify: tests for each.
- [ ] 1.5 Rename the slash-command requirement to twenty. Verify: `openspec validate --all --strict` passes.
- [ ] 1.6 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
