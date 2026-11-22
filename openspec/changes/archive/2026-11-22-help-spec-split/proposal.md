# Proposal

## Why

Archiving `help-layout` pushed the help requirement past 500 characters, so
`openspec validate --strict` fails on the main spec. No behaviour changes.

## What Changes

- The help requirement's description goes back to its earlier text; all
  its scenarios stay.
- The layout rules from `help-layout` move to their own requirement with two
  scenarios of its own.

## Keyboard and resources

None; spec text only.

## Capabilities

### Modified Capabilities

- `tui`: the help requirement is split in two.
