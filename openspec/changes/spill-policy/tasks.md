# Tasks

- [ ] 1.1 Split spill from trim. Keep `cap` for command, job, and fetch results; add `trim` for read, which names offset and limit and writes nothing. Verify: a 10 KB read creates no output file and says how to continue; a 1 MB command output still spills.
- [ ] 1.2 Allow the output directory in the jail. Canonicalise `out_dir` and accept paths under it in `confined`. Verify: `read` opens a spill file and a job log, while `/etc/hosts` stays refused.
- [ ] 1.3 Prune old output at startup. Delete files under `out` and `out/jobs` older than seven days, ignore errors, and keep fresh files. Verify: an eight-day-old file is removed and a one-day-old file stays.
- [ ] 1.4 Rewrite the two prompt lines. Name the spill path, say read or bash search, and state that only file tools are jailed. Verify: the prompt test checks both lines and the absence of "working directory".
- [ ] 1.5 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
