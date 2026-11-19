# Tasks

- [ ] 1.1 Add the memory buffer. Store command, job-wait, and fetch output over 4 KB, hand out `imp://out/<n>`, cap at 16 MB, and evict the least recently used. Verify: two outputs get distinct ids, eviction drops the oldest, and a single 1 MB output fits.
- [ ] 1.2 Read buffers. Parse `imp://out/<n>`, apply offset, limit, line numbers, and the byte ceiling, and report an expired id. Verify: a slice matches the stored text and a missing id says it expired.
- [ ] 1.3 Stop buffering reads. Give `read` a trim path that returns head and tail with an offset hint and writes nothing. Verify: a 10 KB read creates no id and names offset and limit.
- [ ] 1.4 Keep job logs readable. Allow `read` under the output directory and prune `out/jobs` files older than seven days at startup. Verify: a job log opens, an eight-day file is removed, and a one-day file stays.
- [ ] 1.5 Rewrite the prompt lines. Name the buffer address, say read a range, and state that only file tools are jailed. Verify: the prompt test checks both lines and the absence of "working directory" and "spill".
- [ ] 1.6 Clear buffers on `/new` in both front-ends. Verify: an address from before the reset reports expiry afterwards.
- [ ] 1.7 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
