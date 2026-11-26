# Picker style

## Why

Every picker draws a rounded border, a bottom key-hint edge, a rule under the filter, and a `▸` cursor.
The chosen look is quieter: no border, one title row that doubles as the filter and carries `esc` at its
right, a `■` marker on the current entry, and a full-width highlight with a left accent bar on the cursor
row. The picker is one shared type, so one change restyles them all.

## What Changes

- Draw every picker as a borderless centred block: no rounded corners, no side walls, no bottom edge.
- Put the filter in the title row. While the filter is empty, the title row shows the picker title as a
  muted placeholder. The scroll position and `esc` sit at the right of that row.
- Remove the rule under the title row.
- Mark the current entry with `■` in a left gutter column. Keep the `current` word in the right column.
- Mark the highlighted row with a left accent bar and paint it across the full width.
- Show row-action keys in the title row, before `esc`, instead of a footer.
- Remove the bottom key-hint line from every picker. The `/cd` and `/image` browsers keep their path
  title as the placeholder.

## Out of Scope

- Key handling, filtering, matching, sorting, and row content. None of these change.
- Picker sizing (40 to 72 columns, 80% cap) and the eight-row window. The reference image shows more rows;
  raising the cap is a separate decision.
- Colours beyond the existing theme roles. The new look uses the active theme.
- Mouse behaviour other than the footer hint targets, which move to `esc` in the title row.

## Impact

- Specs: `openspec/specs/picker/spec.md`. Two requirements are replaced (bordered overlay, frame labels),
  and three are modified (current marker, row shortcut hints, directory key hints).
- Code: the picker frame and footer in `src/tui.rs`, and the tui tests that assert borders or footer text.
- Users see the new look in every picker. Key hints are no longer shown on screen.
