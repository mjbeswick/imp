# Tasks

- [x] 1.1 Add `Log::fork`. Copy the transcript, allocate a new id, write a sidecar titled `fork of <source>`, and leave the source untouched. Verify: bytes match, ids differ, the source files still exist, and the sidecar carries cwd and model.
- [x] 1.2 Add `Session::fork`. Require an attached log, attach the copy, log the source id, and return both ids without touching history. Verify: history length is unchanged, later appends land in the fork, and no log returns an error.
- [x] 1.3 Wire `/fork` in chat and line mode. Chat sends `Cmd::Fork` and updates the id used at exit; line mode prints both ids. Verify: command tests see the fork, and a line-mode run appends to the new file.
- [x] 1.4 Update the command list and docs after the preceding command changes land. Verify: `/help` lists `/fork`, the count matches, and the README documents it.
- [x] 1.5 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
