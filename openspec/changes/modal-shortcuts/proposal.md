# Modal shortcuts and session layout

## Why

Pickers show their keys in a bottom hint line or in the title row. The reference for the sessions modal puts
the actions in a footer bar with their keys, groups sessions by day, and shows the highlighted session's
directory. Row actions use single letters, which collide with the filter. Ctrl-key actions and one footer bar
fix both, and apply to every modal that declares row actions.

## What Changes

- Every picker with row actions draws a footer bar inside its box: a panel-filled row listing each action as
  its name and key, the first action at the left and the rest at the right.
- Row actions use Ctrl-key shortcuts, such as `ctrl+r` rename and `ctrl+d` delete, so every letter types into
  the filter. **BREAKING** for single-letter row actions: `r`, `d`, `n`, `u`, and `e` stop working inside
  pickers. They are replaced by the Ctrl keys in the table in design.md.
- The session picker groups sessions under day headings: `Today`, then each earlier day's full date. Headings
  are not selectable, and moving the highlight skips them.
- The session picker's right column shows the highlighted session's directory only. The message count is no
  longer drawn. **BREAKING** for the session list's right column.
- `ctrl+a` in the session picker filters the list to sessions in the current directory.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `picker`: row shortcut actions, the picker columns for sessions, a new footer bar, and a new day grouping for
  the session picker.

## Out of Scope

- Confirmation prompts, the question prompt, provider forms, and settings forms. They keep their own hints.
- The command menu, which has no row actions.
- The session state marks. They keep their current glyphs and rules.
- Keyboard behaviour outside an open picker. Ctrl+D quitting the app and Ctrl+R retrying a turn still work when
  no picker is open.

## Impact

- Code: `src/tui.rs` (the row-action table in `Picker`, the key routing for open pickers, the overlay's footer
  drawing, `session_picker`, and `session_row` in `src/store.rs` for the day field).
- Tests: the picker tests that assert single-letter actions and the footer hint text.
- Specs: `openspec/specs/picker/spec.md`, through a delta in this change.
- Sequencing: this change's title-row footer rule depends on `picker-style` being archived first. See design.md.
