# Composer style

## Why

The composer is one bare row with a `> ` glyph, and the status is one dim line of model, tokens and cost.
The chosen look groups the composer into a filled block with an accent bar, shows the agent, mode, model,
provider and thinking level in that block, moves the token figures and the command hint onto a status bar
below it, and attaches the slash autocomplete to the composer as a menu. The picker panel already has the
same fill and accent language, so the composer and menu can share it.

## What Changes

- Draw the composer as a filled block with the same derived panel background as the picker, and a
  left accent bar in the theme's accent colour.
- Give the composer block a blank row above the input and a blank row below it, then a footer row:
  the agent name in the accent colour, the permission mode, the model, the provider in muted text, and
  the thinking level in the accent colour, separated by ` · `.
- Drop the `> ` prompt glyph. The caret sits at the start of the input text.
- Move the status line below the composer block: on the left, an activity marker with `esc interrupt`
  while a turn streams; on the right, the context tokens with their percentage of the window, the cost,
  and `ctrl+p commands`.
- Show the count of running commands on a muted row above the composer, only while one runs.
  **BREAKING** for the status line format: `3.4K tok` becomes `3.4K (3%)`, and the model name moves from
  the status line to the composer footer.
- Attach the slash autocomplete to the composer: it renders as a menu directly above the composer block,
  across the full width, with no title row and no footer. Each row shows the command name and a muted
  description. The highlighted row takes the select background. The menu keeps its filter, its ranking and
  its keys. **BREAKING** for the autocomplete: the title row and the Enter and Esc footer go.
- Remove the rule that drew above the input, and the single-bordered-overlay rule that no longer matches
  the borderless pickers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tui`: the layout requirement, the status line requirements (tokens and cost), the slash autocomplete
  requirement, and the composer growth requirement, where the block and the rows it draws change.

## Out of Scope

- Keys, editing, history, queueing, steering and shell mode. None of these change.
- Picker panels other than the slash menu. Those are covered by `picker-style`.
- Provider forms, question prompts and confirmations. They keep their current layout.
- Theme colours. The composer uses the existing theme roles and the panel derived by `picker-style`.
- The count of running commands beyond showing it. Its source is confirmed in design.md before tasks.

## Impact

- Code: the composer and status drawing in `src/tui.rs` (`composer`, `paint_composer_row`,
  `prompt_agent`, `prompt_width`, `paint_status`, `status_line`), and the slash menu's overlay
  (`sync_menu`, and the picker overlay for the command kind).
- Tests: the tui tests that assert the `> ` glyph, the status line text, the title and footer of the
  command picker, and the rule above the input.
- Specs: `openspec/specs/tui/spec.md`, through a delta in this change.
- Users see the new composer, status bar and menu. Keyboard behaviour is unchanged.
