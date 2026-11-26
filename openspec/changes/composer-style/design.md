# Design

## Context

Citations are to `main` at `49c1b217`, in `src/tui.rs` unless a path is given.

- The composer is `composer` (around lines 7294–7390). It draws the input one row per wrapped line, with
  a glyph from `prompt_agent` and `prompt_width`: the agent name shows only when more than one agent exists
  (7424–7431), then `> `, `» ` for steer or queue, or `$ ` for a shell draft (7367–7377). `paint_composer_row`
  (7393–7420) paints the glyph in the user role.
- The status line is one dim row. `paint_status` (7579–7606) adds the unseen and waiting counts and the
  context percentage from `crate::context::percent`. `status_line` (7608 onward) builds the text: model,
  served model, thinking, cost from `crate::pricing::cost` for DeepSeek only, and the permission mode.
- The slash autocomplete is the command picker. `sync_menu` (4299–4322) opens `Picker::new("commands", …)`
  as a pending overlay. The overlay (the picker drawing code after `fn overlay`) draws it centred, with a
  title row (picker-style), and the footer hint comes from `Picker::hints`.
- The palette's derived panel (`Palette::panel`, added by `picker-style`) gives the fill for the composer
  block, so the composer and the picker share one background.
- Running background commands are in `src/jobs.rs`: `Job::running` (31) and `Jobs::poll` (181). The "1 command"
  row can count `Job::running` entries.
- No binding for Ctrl+P exists in `src/tui.rs` (a search for `Ctrl+P` and `Key::Ctrl('p')` finds none).
- The current spec wording is in `openspec/specs/tui/spec.md`: the layout requirement (line 282), the token
  and cost requirement, the slash autocomplete requirement (568), and the composer growth requirement (708).

## Decisions

1. **Composer block.** The block is the input rows with a blank row above and below, a full-width panel fill,
   and a left accent bar in `Slot::Accent`. The block sits above the status bar. The prompt glyph goes; the
   caret sits at the input's start.
2. **Footer row.** The row under the input reads `<agent> <mode> · <model> <provider> · <thinking>`. The
   agent is always shown (today it shows only with several agents), in the accent colour. The mode is always
   shown (today only when it is not `normal`). The provider sits in muted text. The thinking level is in the
   accent colour.
3. **Status bar.** One row below the block. Left: `esc interrupt` while a turn streams, with a dotted marker
   in the accent colour. Right: the tokens in the form `262.2K (26%)`, the cost, and `ctrl+p commands`.
   The spinner stays off (existing requirement).
4. **Running commands row.** A muted row above the block shows `⁝ N command` or `N commands`, counted from
   `Job::running`, and is absent when no job runs. Judgement call: the source is background jobs, not tool
   calls, because the job list is the only running-command state in the code. Cost if wrong: the row counts
   the wrong thing; the source is one function.
5. **Ctrl+P.** The status bar hint says `ctrl+p commands`, so Ctrl+P must open the command menu. Judgement
   call: add the binding (it is a new key, so the keys requirement changes). The alternative is to drop the
   hint. Cost if wrong: one key and one hint change.
6. **Autocomplete menu.** The command picker draws above the composer block at full width. It has no title
   row and no footer. Rows are the command name, then a muted description, the highlight takes the select
   background and the accent bar. The menu scrolls within the fixed window of picker-style. The filter,
   ranking and keys are unchanged.
7. **Layout.** The rule above the input goes, the one-bordered-overlay rule is replaced by "no border", and
   the panel fill is the only background drawn besides the highlight.

## Risks and assumptions

- Assumption: the accent colour in the default theme is close to the orange in the reference. If it is not,
  the accent bar and the thinking level read in a different hue.
- Assumption: `auto` in the reference is the permission mode. The mode names in code are `normal`, `strict`,
  `yolo` and the auto-mode change; the label is the current mode.
- Risk: the status bar's left side can carry a toast or a retry message. Those keep their place and push the
  activity marker out, which the status line already does.
- Risk: the tests that check the `> ` glyph and the status text change in many places. They move with the code.
- Open question for the user: is the running-command row about background jobs (assumed) or about tool calls?
- Open question for the user: is the Ctrl+P binding wanted (assumed), or should the hint be removed?

## Expected bytes

The change adds the footer, the status bar and the menu layout, and removes the glyph and the rule. The size
change is measured with `./scripts/size.sh` after implementation; no fixed figure is claimed here.
