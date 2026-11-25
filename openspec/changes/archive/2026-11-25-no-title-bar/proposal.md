# Proposal

## Why

The top row spends a whole line on the session name, the session position, and
a clock. The transcript can use that row, and the name already belongs in the
terminal title.

## What Changes

- Drop the title bar row. The transcript starts at the top row.
- Set the terminal title to `imp` before the first message and `<title> · imp`
  after it, updating on startup, session switches, and renames.
- Show `n/m` at the start of the status line while more than one session is
  live, not on a row of its own.
- Draw the rule above the composer only when it carries a label or a warning
  tint, never as a bare divider.
- Remove the clock and the once-a-minute repaint it drove.
- Update the docs and the specs that named the title bar or the bare rule.

## Keyboard and resources

No keys. One OSC 0 write when the name changes; no timer.
