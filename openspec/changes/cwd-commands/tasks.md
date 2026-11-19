# Tasks

- [x] 1.1 Add the shared root resolver. Expand a leading `~`, join relative paths, canonicalise, and reject a missing path or a file. Verify: tests for `~`, `~/.`, `..`, an absolute path, a missing path, and a regular file.
- [ ] 1.2 Add `Session::set_root`. Set the tools root, reload skills, memory, and the system prompt, update the sidecar's `cwd`, and log a `cwd` event. Verify: after the call, a read tool is jailed to the new root, the first system message names the new directory, and the sidecar reads back the new path.
- [ ] 1.3 Add `/cwd` and `/cd` to the interface. Resolve against the App root, send `Cmd::SetRoot`, apply `Ev::Root`, and keep typed `!` commands in step. Verify: `/cwd` shows the root, `/cd ..` changes it, a missing path leaves it unchanged, and `!pwd` prints the new root.
- [ ] 1.4 Add line mode and resume precedence. `/cd` calls `Session::set_root`; resume uses `--cwd`, then the sidecar `cwd`, then launch, falling back with a note when the recorded directory is gone. Verify: a resumed session lands in its recorded directory, `--cwd` overrides it, and a deleted directory falls back.
- [ ] 1.5 Document the commands. Add both to `help::COMMANDS` and the README, and update the command-count expectations. Verify: `/help` lists twelve commands, and the help tests name `/cwd` and `/cd`.
- [ ] 1.6 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
