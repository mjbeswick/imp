# Proposal

## Why

`/rename` shipped with `more-commands` and appears in `/help`, but the archived
command requirement still says nineteen commands and omits it. The spec is a
contract, so it must name the twentieth command and its behaviour.

## What Changes

- The command requirement becomes twenty commands, including `/rename`.
- A scenario states that `/rename <title>` retitles the current session.

## Capabilities

### Modified Capabilities

- `tui`: the authoritative command list and the rename scenario.

## Impact

- `openspec/specs/tui/spec.md` only. No source changes.
