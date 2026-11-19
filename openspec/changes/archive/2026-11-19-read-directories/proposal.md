# Proposal

## Why

Finding what exists costs more turns than reading it. The model runs `ls`,
`find`, or `tree` with bash, waits for the result, then reads the files it
discovered. `read` already accepts several files in one call, but it refuses a
directory, so discovery cannot share that call.

## What Changes

- Let a `read` path be a directory. It returns a sorted tree of names, with
  directories marked and contents not read.
- Add `depth` to the call, default 1 and capped at 3, and cap each directory at
  200 entries with a note when cut.
- Skip `.git`, `.jj`, `target`, `node_modules`, and the usual build directories.
- Allow files and directories in the same call, so one request can list a tree
  and return file contents.
- Update the tool description and add one prompt line telling the model to ask
  for several paths at once.

## Capabilities

### Modified Capabilities

- `tools`: `read` gains directory listings, depth, and noise filtering while
  keeping its file, line-number, and cap behaviour.

## Impact

- `src/tools.rs`: the read path walker, sorting, ignores, and caps.
- `src/agent.rs`: the `depth` argument and the tool description.
- `src/prompt.rs`: one line on batching paths.
- `openspec/specs/tools/spec.md`: the read requirement and its scenarios.
