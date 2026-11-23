# Proposal

## Why

Line mode prints a blank line before every tool, info and error row, even
when the output before it already ended a line, and the resume hint repeats
`imp:` after a command the user just typed as `imp`.

## What Changes

- A row starts a new line only when the output before it stopped mid-line.
- The exit hint reads `resume with imp -r <id>`.

## Keyboard and resources

None.
