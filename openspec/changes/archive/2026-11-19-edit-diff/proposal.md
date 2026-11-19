# Proposal

## Why

File changes are invisible. An edit row says `✓ edit src/tui.rs (2 changes)`,
and even Ctrl+O shows only that sentence. The user has to open the file or run
`git diff` to see what the agent actually did, which defeats the point of a
single-column interface.

The edit call already carries the exact old and new strings, so the change is
known without a diff algorithm.

## What Changes

- After a successful `edit` or `write`, draw a coloured diff under the tool
  row: removed lines red, added lines green.
- Show at most 12 diff lines by default with the remainder counted; Ctrl+O
  shows the full diff.
- Keep the diff out of the tool result, so the model does not pay tokens to
  read back what it sent.
- Fall back to `-` and `+` prefixes with no colour when colour is off.
- Document the behaviour.

## Capabilities

### Modified Capabilities

- `tui`: a tool row for a file change can carry a coloured diff, capped by
  default and expandable with Ctrl+O.

## Impact

- `src/agent.rs`: build the user-only diff detail from edit pairs or write
  content and pass it with the tool-end event.
- `src/tui.rs`: a detail field on the tool row, per-line painting, the cap, and
  expansion.
- `src/main.rs`: print the same detail in line mode.
- `openspec/specs/tui/spec.md`: one requirement for file-change diffs.
