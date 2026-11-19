# Proposal

## Why

The prompt makes two promises the code breaks. Spill files and job logs live
under `~/.local/state/imp/out`, which the path jail does not allow, so `read`
refuses every path the prompt tells the model to read. And "paths are confined
to the working directory" implies `bash` shares the jail, while `bash` can reach
anywhere.

Writing command output to disk also creates litter with no cleanup, and a large
`read` result is written to a file even though the source file is still there.

## What Changes

- Keep command and fetch output over 4 KB in a buffer addressed as
  `imp://out/<n>`, held in memory up to 16 MB. When memory is full, demote the
  least-recently-used buffer to an unlinked temporary file; cap that spill at
  64 MB and drop the oldest file-backed buffer past it.
- Let `read` accept that address with offset and limit, and report an expired
  buffer when the id is gone.
- Stop buffering `read` results: a long read returns a head and tail with a
  hint to use offset and limit.
- Keep job logs on disk, allow `read` to open them, and delete them after seven
  days.
- Clear buffers on `/new`, and lose them on exit; a resumed session cannot
  reopen a command output from before the restart.
- Rewrite the two prompt lines: buffered addresses are read with `read`; only
  `read`, `edit`, and `write` are jailed, and `bash` is not.

## Capabilities

### Modified Capabilities

- `tools`: large command and fetch results are buffered in memory, `read`
  accepts `imp://out/<n>`, read does not buffer, and job logs stay readable
  with a retention rule.

## Impact

- `src/tools.rs`: the buffer store, eviction, virtual reads, and read trimming.
- `src/prompt.rs`: the two limits lines.
- `src/agent.rs` and `src/main.rs`: clear buffers on new sessions.
- `openspec/specs/tools/spec.md`: buffering and jail scenarios.
