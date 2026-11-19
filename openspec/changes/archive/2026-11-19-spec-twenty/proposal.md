# Proposal

## Why

`/rename` shipped with `more-commands` and appears in `/help`, but the command
requirement still says nineteen commands and omits it. The spec is a contract,
so it must name the twentieth command.

## What Changes

- The command requirement becomes twenty commands, including `/rename`.
- A scenario states that `/rename <title>` retitles the current session.
- The empty `spec-rename` archive is removed.

## Capabilities

### Modified Capabilities

- `tui`: the authoritative command list and the rename scenario.

## Impact

- `openspec/specs/tui/spec.md` and the archive directory. No source changes.
