# Tasks

- [ ] 1.1 Remove legacy spill files and cap the directory. Delete top-level `out-*` at startup, keep job logs for seven days, and delete the oldest job logs above 64 MB. Verify: a fresh legacy file is removed, a fresh job log stays, an eight-day log is removed, and a small cap drops the oldest file.
- [ ] 1.2 Remove the current leftovers and document the rules. Verify: only recent job logs remain under `out/`, and the README states the seven-day and 64 MB limits.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
