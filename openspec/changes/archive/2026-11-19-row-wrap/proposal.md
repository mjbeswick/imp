# Proposal

## Why

Tool rows do not share an indent: a successful row starts at column one,
while an ask row and a failure preview start two columns in. Long bash
summaries are cut at 120 characters with an ellipsis instead of wrapping, so
the command and its duration are lost halfway across the row. The ask question
is flattened onto one clipped line instead of wrapping beneath its action.

## What Changes

- Every tool row starts with its mark in column one and puts its summary after
  one space; every wrapped continuation, failure preview, question, and option
  aligns two columns in.
- Tool summaries keep the whole command or path. The renderer wraps them to the
  terminal width instead of cutting them at 120 characters.
- The ask row shows its question and options as wrapped lines under the action,
  not one flattened line.
- Durations close the summary line they belong to; nothing is cut with an
  ellipsis for width alone.

## Capabilities

### Modified Capabilities

- `tui`: tool rows share one indent and wrap their content instead of
  truncating it.

## Impact

- `src/tui.rs`: row prefixes and wrapping.
- `src/agent.rs`: the summary builder stops flattening to 120 characters.
- `openspec/specs/tui/spec.md`.
