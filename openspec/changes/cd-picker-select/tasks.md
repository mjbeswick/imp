# Tasks

- [ ] 1.1 Add the `./` row and switch the directory picker keys: Enter chooses, Tab and Right open, Left goes up; update the footer. Verify: tests for the row order, Enter, Tab, Right, and choosing `./` in a leaf folder.
- [ ] 1.2 Let the image picker open folders with Tab and Right as well as Enter, and update its footer. Verify: a test that Right opens a folder and Enter attaches a file.
- [ ] 1.3 Report a root change once, with the cache note on the same line. Verify: a test counts one `cwd:` line after choosing.
- [ ] 1.4 Update `README.md` and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
