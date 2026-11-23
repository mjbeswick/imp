# Proposal

## Why

The agent list shows `default`, a built-in with no instructions, and
`example`, a reviewer sample `imp init` writes. Neither is a designed default.
Work usually runs build, plan, then review.

## What Changes

- Three built-in agents: `build` (all tools, the starting agent), `plan`
  (read and bash; investigates and writes a plan, never edits) and `review`
  (read and bash; reports defects in a diff).
- A user file with the same name replaces a built-in.
- `default` goes. `--agent default` and a saved `default` resolve to `build`.
- `imp init` stops writing `agents/example.md`. An existing copy is the
  user's file and stays.

## Keyboard and resources

Tab cycles build, plan, review: three stops. Built-ins are compiled in, about
1 KB of text.
