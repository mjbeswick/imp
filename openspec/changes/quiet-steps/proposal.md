## Why

The coding-agent review found three small frictions. Tool rows print absolute paths that eat a 65-column screen. `git ls-files` asks although it only lists. The model writes "Now let me run the tests" between steps, which costs tokens and rows.

## What Changes

- Tool rows show paths inside the session directory relative to it, and the scratch folder as `$IMP_TMP`.
- `git ls-files` runs without a prompt.
- The baseline prompt says to write nothing between tool calls.

## Impact

- `src/agent.rs`, `src/tui.rs`, `src/perm.rs`, `src/prompt.rs`.
