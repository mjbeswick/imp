# Proposal

## Why

A hands-on review turned up four problems with tool rows and declines:

- A command that ran and exited non-zero gets the same red `✗` as a real
  failure. `ls; cat calc.py; ls tests` failed only on the missing `tests`,
  yet read as an error, with its output squashed onto one line.
- Ctrl+E expands only the newest call, so an earlier `✗` cannot be opened.
  The expanded view shows the raw JSON arguments.
- After the user declines a command, the next suggestion asks why it was
  refused, or offers to do it again.
- A decline is said twice: once on the tool row, once as an info row.

## What Changes

- Four finishes instead of two: `✓` done, `!` ran and exited non-zero in the
  accent colour, `✗` declined in the muted colour, and `✗` failed in the
  error colour.
- A non-zero exit shows the last three lines of its output under the row,
  line breaks kept, instead of one flattened line.
- Ctrl+E expands or collapses every tool row of the latest turn. A click on
  one of those rows toggles just that row.
- Expanded rows show `$ command` for a shell call and `key: value` lines for
  other tools, not JSON.
- No suggestion follows a turn in which the user declined something.
- A decline shows once, on its tool row, as `declined`. The model's text is
  unchanged.
- Line mode uses the same four finishes.
- The spec's stray Ctrl+O for expanding a call becomes Ctrl+E, as the keys
  already are.

## Keyboard and resources

No new keys. The latest turn's rows keep their arguments and up to 20 lines
of output each, replaced when the next turn starts.

## Capabilities

### Modified Capabilities

- `tui`: tool row finishes, previews, expansion, and declines.
- `agent-loop`: no suggestion after a declined turn.
