# Tasks

- [x] 1.1 Paint inline code bold in the tool colour and keep fences dim. Verify: a frame test with colour on shows `\x1b[1m` around the code span and no dim attribute, while a fenced block stays dim.
- [ ] 1.2 Show a shell draft: `$ ` prompt and `shell · local` status. Verify: frame tests for a draft starting with `!` and for one that does not.
- [ ] 1.3 Name `!` in the key list. Verify: `/help` mentions it.
- [ ] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
