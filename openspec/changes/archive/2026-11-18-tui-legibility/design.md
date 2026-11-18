# Design

## Approaches

1. **Reuse roles, not new ones.** Inline code paints with `Role::Tool` and a bold
   attribute; a shell draft swaps the prompt glyph and the status text. Chosen:
   no new colour role, no new mode, and the same render path.
2. **A dedicated `Code` role.** A seventh colour to tune and a spec change to
   justify, for a difference the tool colour already carries.
3. **A shell mode with state.** `!` becomes sticky until Esc. More state to test,
   and a stray `!` would silently reroute the next message.

## Decisions

- `code` spans paint with `Role::Tool` plus `\x1b[1m`. `dim` spans keep
  `Role::Thinking`. `**bold**` and inline code differ by colour, not weight.
- Fenced blocks stop setting `code: true`, so a code block is not bold on every
  line. It stays dim.
- The composer prefix is `$ ` when the first row starts with `!`, `> `
  otherwise. The prefix is painted with `Role::User` as now.
- The status row, while a shell draft is open, reads `shell · local · not sent
  to the model` in the thinking colour. It returns to model and cost as soon as
  the draft stops starting with `!`.
- No mode, no keybinding. Typing `!` is the whole affordance.
