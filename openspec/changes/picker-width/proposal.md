# Proposal: picker width follows its content

## Why

The picker modal is capped at 72 columns on wide terminals. Pickers with long descriptions, such as
`/keys`, cut each description with an ellipsis while the terminal has room to show it.

## What Changes

- A picker's width grows to fit its widest row, when the terminal is wide enough.
- The width is at least 72 columns and at most 120 columns, and never more than 90% of the terminal width.
- Narrow terminals, under 60 columns, keep their current width.
- The provider form keeps its current width.

## Out of Scope

- Changing the picker's layout, its colours, or how a row is drawn.
- Changing the provider form's width.
- Wrapping descriptions onto more lines.

## Impact

- `src/tui.rs`: `modal_width` takes the width the content wants, and the picker overlay passes it.
- `src/tui/tests.rs`: tests for the width rule at several terminal sizes.
- `README.md`: nothing; the change is visual only.
