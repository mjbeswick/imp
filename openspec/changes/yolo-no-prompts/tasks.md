# Tasks

- [x] 1.0 Check the deny list, `allow_sudo`, and confirmation before a privileged command runs attached; today the attached run happens first. Verify: an agent test with `allow_sudo=false` never reaches `run_attached`.
- [x] 1.1 Allow own-config edits and `sudo` without a prompt in `yolo`. Verify: `perm` tests for each mode.
- [ ] 1.2 Skip memory and compaction save prompts in `yolo`. Verify: an agent test counts zero confirmations in `yolo` and one in `normal`.
- [ ] 1.3 Paint the prompt glyph and rule in the error colour in `yolo`, in both front-ends. Verify: colour frame tests for `yolo` and `normal`.
- [ ] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
