# Proposal

## Why

Choosing a theme means applying one, looking, and opening the picker again.
The screen should show each theme as the highlight moves.

## What Changes

- While the theme picker is open, the whole screen repaints in the
  highlighted theme. Nothing is saved.
- Enter applies and saves the highlighted theme.
- Esc, Ctrl+C, or leaving the picker any other way restores the theme that was
  active when it opened.

## Capabilities

### Modified Capabilities

- `tui`: the theme picker previews.

## Impact

- `src/tui.rs`.
