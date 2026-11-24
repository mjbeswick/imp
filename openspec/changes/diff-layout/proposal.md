# Proposal: diff layout

## Why

File-change diffs are drawn inline, one line after another, even on wide terminals where
two columns would show the change more clearly. On narrow terminals inline is the only
layout that fits.

## What Changes

- On a wide terminal, the diff under an `edit` or `write` row is drawn side by side: old
  text on the left, new text on the right, with removed and added lines in the delete and
  add colours.
- On a narrow terminal, the diff stays inline, as it is today.
- The switch point is one width threshold, set in the design, applied when the terminal is
  drawn.

## Out of Scope

- Changing what a diff contains, the 12-line preview, or the Ctrl+E expansion.
- Changing the diff the model sees. The diff stays out of the tool result.
- User settings for the layout. The width decides it.
