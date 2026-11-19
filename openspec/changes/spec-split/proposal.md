# Proposal

## Why

Archiving `help-format` and `tui-polish` left four requirements over the 500
character limit, so `openspec validate --all --strict` fails on the main specs.

## What Changes

- Split each long requirement into a shorter one that keeps its scenarios and
  a new one that carries the moved clauses.
- No behaviour changes; no source changes.

## Capabilities

### Modified Capabilities

- `tui`: the chat column and help requirements.
- `picker`: the value picker and overlay requirements.

## Impact

- `openspec/specs/tui/spec.md`, `openspec/specs/picker/spec.md`.
