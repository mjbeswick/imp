# Proposal

## Why

The model is told the terminal width but not its height, so it cannot size a
picture or chart to the screen. chafa ignores `COLUMNS` and `LINES` when its
output is a pipe, so sizes must be passed by the command.

## What Changes

- The environment note names the width and height, and is sent again when
  either changes.
- The agent's commands get `COLUMNS` and `LINES` set to the live terminal size,
  so `chafa --size "${COLUMNS}x${LINES}"` fits the screen at the moment it runs.

## Capabilities

### Modified Capabilities

- `tui`: the size note carries the height.
- `tools`: agent commands see the screen size.
