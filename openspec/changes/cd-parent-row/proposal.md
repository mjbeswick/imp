# Proposal

## Why

The directory browser can climb with Left, but nothing on screen says so. A
`..` row at the top of the list makes the parent reachable with the same keys
as every other directory and shows where the climb starts.

## What Changes

- Every directory listing begins with a `../` row, except the filesystem root,
  which has no parent.
- Enter on `../` opens the parent directory; Tab selects it as the session
  root.
- The row follows the same rules as any directory row, including climbing above
  the session root.
- The image browser is unchanged.

## Capabilities

### Modified Capabilities

- `picker`: the directory list gains a leading parent row.

## Impact

- `src/tui.rs`: directory items and tests.
- `openspec/specs/picker/spec.md`.
