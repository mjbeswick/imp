# Tasks

- [ ] 1.1 Add the table helper and rebuild `usage` on it. Normalise option syntax, align each section, and wrap to the width. Verify: a 60-column render has no overflow and every description starts in its section's column.
- [ ] 1.2 Rebuild `/help` with the same helper. Move keys into rows, group commands by area, and pass the terminal width. Verify: chat, line mode, and `--help` name the same flags and twenty commands.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
