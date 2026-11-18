# Proposal

## Why

A tool call costs two rows and shows raw JSON: `bash  {"command": "cd ... && cat
... ; echo ---; ..."}`, then `✓ exit 0 <output>`. A refused write reported a
tick, because success was inferred from the result text instead of the tool's
own flag. The elapsed time includes the wait for a confirmation, so a command
that took 3 ms read as 30,997 ms. The spec already promises one row with a short
summary, the duration, and expansion, and the expansion key does not exist.

## What Changes

- Summarise each call by tool: the command, the path and line count, the query,
  the question.
- Draw one row per call: a spinner while it runs, then `✓` or `✗` with the
  duration. Update that row instead of appending a second.
- Time the tool itself, so the confirmation wait is not counted.
- Report the tool's own success flag, so a refusal is a failure with its reason.
- Add Ctrl+O to expand the last call: full arguments and output.
- Allow writes under imp's own config directory, where skills and agents live,
  and keep the jail for everything else.
- Tell the model that `/reload` re-reads skills, so it stops saying restart.

## Capabilities

### Modified Capabilities

- `tui`: tool activity is one summarised row that updates in place and expands.
- `permissions`: the jail leaves imp's own config directory writable.

## Impact

- `src/tui.rs`: tool rows, Ctrl+O, expansion.
- `src/agent.rs`: per-tool summary, honest success flag, timing.
- `src/tools.rs`: jail exception, timing.
- `src/prompt.rs`: the reload note.
- Spec deltas under `tui` and `permissions`.
