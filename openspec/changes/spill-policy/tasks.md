# Tasks

- [x] 1.1 Add the buffer store. Memory first, demote the least-recently-used buffer to an unlinked temp file when 16 MB fills, cap spill at 64 MB, and hand out `imp://out/<n>`. Verify: a demoted id still reads, no temp file is visible, and passing the spill cap expires the oldest.
- [x] 1.2 Read buffers. Parse `imp://out/<n>`, serve memory and file-backed buffers with offset, limit, line numbers, and the byte ceiling, and report an expired id. Verify: a slice matches from both backends and a missing id says it expired.
- [x] 1.3 Stop buffering reads. Give `read` a trim path that returns head and tail with an offset hint and writes nothing. Verify: a 10 KB read creates no id and names offset and limit.
- [x] 1.4 Keep job logs readable. Allow `read` under the output directory and prune `out/jobs` files older than seven days at startup. Verify: a job log opens, an eight-day file is removed, and a one-day file stays.
- [x] 1.5 Rewrite the prompt lines. Name the buffer address, say read a range, and state that only file tools are jailed. Verify: the prompt test checks both lines and the absence of "working directory" and "spill".
- [x] 1.6 Clear buffers on `/new` in both front-ends. Verify: an address from before the reset reports expiry afterwards.
- [ ] 1.7 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
