# Proposal

## Why

The prompt makes two promises the code breaks. Spill files and job logs live
under `~/.local/state/imp/out`, which the path jail does not allow, so `read`
refuses every path the prompt tells the model to read. And "paths are confined
to the working directory" implies `bash` shares the jail, while `bash` can reach
anywhere.

Spilling also duplicates work: a large `read` result is written to a file even
though the source file is still there and can be reopened with an offset.

## What Changes

- Allow `read` to reach the session's output directory, so spill files and job
  logs are readable as the prompt says.
- Spill only results that exist nowhere else: `bash`, `job wait`, and `fetch`.
  A large `read` result returns a head and tail with a hint to use offset and
  limit, and writes no file.
- Delete output files older than seven days when a session starts.
- Rewrite the two prompt lines: spill paths are read with `read` or searched
  with `bash`; only `read`, `edit`, and `write` are jailed, and `bash` is not.
- Update the tools spec for the narrowed spill, the readable output directory,
  and retention.

## Capabilities

### Modified Capabilities

- `tools`: spill applies to command, job, and fetch results, `read` does not
  spill, the output directory is inside the jail, and old output is pruned.

## Impact

- `src/tools.rs`: `trim` for read, the jail exception, and pruning.
- `src/prompt.rs`: the two limits lines.
- `src/main.rs`: prune at startup.
- `openspec/specs/tools/spec.md`: spill and jail scenarios.
