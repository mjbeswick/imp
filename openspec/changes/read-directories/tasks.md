# Tasks

- [ ] 1.1 List directories from `read`. Resolve through the jail, sort by name, mark directories with `/` and symlinks with `@`, and never follow links. Verify: a temp tree lists in order with the right markers, and a symlink loop returns without hanging.
- [ ] 1.2 Add depth, ignores, and caps. Default depth 1, clamp to 3, skip the six noise names, cap 200 entries per directory and 3.5 KB overall, and print a cut note. Verify: depth 2 nests children, `.git` and `target` are absent, 500 entries show 200 plus a note, and a deep tree stays under 4 KB.
- [ ] 1.3 Update the schema and prompt. Describe directories and `depth` in the `read` tool, pass the argument through dispatch, and add the batch-paths sentence to the prompt. Verify: a mixed call returns a listing and numbered file in one result.
- [ ] 1.4 Document the behaviour in the README tools section. Verify: the read bullet names directories, depth, and the noise list.
- [ ] 1.5 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
