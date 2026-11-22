# Proposal

## Why

At 54 columns, the Hackberry's width, `imp --help` overflowed: the width was
clamped to at least 60. Defaults split across rows, long-only flags sat in
the short-flag column, long examples broke the column, and nothing was
coloured on a terminal.

## What Changes

- The width floor drops to 40. Below 24 free columns, every description goes
  under its cell, indented six.
- Long-only flags are indented past the `-x, ` column.
- A `(default: X)` takes its own row under the description.
- Examples put their description above the command as a dim `# ` line.
- On a terminal without `NO_COLOR`, headings are bold, cells accent, and
  defaults and example notes dim. `/help` in the chat stays plain.
- Value names lose their angle brackets (`--model NAME`); optional ones keep
  square brackets (`[ID]`). The title loses its em dash.

## Keyboard and resources

No keys. No runtime cost beyond formatting the text once.

## Capabilities

### Modified Capabilities

- `tui`: help layout.
