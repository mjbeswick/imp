# Proposal

## Why

The status line names only the configured model, so a turn served by a
different snapshot is invisible. Tool rows lose their shape when a call fails:
the failure preview is appended after the exit code and wraps without an
indent, and the `✓`/`✗` marks carry no weight against the dim row. There is no
way to see every tool call's arguments and output without pressing Ctrl+O once
per call. Shell commands inherit whatever directory imp happened to start in,
with no flag to choose another.

## What Changes

- Show the served model string from the API response when it differs from the
  configured model name.
- Put a failure preview on its own indented line, align wrapped row text under
  the content, and draw `✓` and `✗` in bold.
- Add `/verbosity high|low`. Low is the default one-line summary; high expands
  every tool call with its arguments and capped output in both front-ends.
- Run `bash` with the session root as its working directory and add
  `-C, --cwd <PATH>` to choose that root at launch.
- Update `/help`, `--help`, and the README for the new command and flag.

## Capabilities

### Modified Capabilities

- `tui`: the status line reports the served model, tool rows format and weight
  their marks, and `/verbosity` controls how much each call shows.
- `tools`: `bash` starts in the session root, and the launch directory can be
  chosen with `--cwd`.

## Impact

- `src/api.rs`: capture the `model` field from streamed chunks.
- `src/tui.rs`: `Status`, `status_line`, `ToolRow`, `tool_row_text`, the frame
  painter, and the command table wiring.
- `src/main.rs` and `src/agent.rs`: pass verbosity through, print detail in
  line mode, and choose the session root from `--cwd`.
- `src/tools.rs`: set the bash working directory.
- `src/help.rs` and `README.md`: the new command and flag.
- `openspec/specs/tui/spec.md` and `openspec/specs/tools/spec.md`.
