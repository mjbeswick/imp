# Design

## Context

Citations are to `main` at `qkuyzvtp`, which holds the merged `picker-style` and `session-goals` work.

- The session picker is `session_picker` in `src/tui.rs`. It lists up to twenty sessions from
  `crate::store::Log::listing`, one `PickItem` per session with `session_row(&m, now)` as its label and
  description, marked by `mark_rows`. The title is `sessions` or `sessions · N of M` when cut.
- Row actions are declared on items as `actions: Vec<(char, &'static str)>`. The session picker declares
  `n new`, `r rename`, and `d delete` (`src/tui.rs` around line 2399). Provider rows declare `u use`, `e
  configure`, `d delete`, and `n new` (around 4160 and 4171). Revoke, add, and allow rows declare one action
  each (around 4089 to 4107).
- `Picker::action(key)` looks up a declared key. `footer()` builds the bottom hint line from the actions; the
  `picker-style` change removes that hint and moves the keys to the title row. This change replaces it with
  a footer bar.
- Global keys collide with the Ctrl keys this change needs. `Key::Ctrl('d')` quits when the composer is empty
  (around line 3739). `Key::Ctrl('r')` retries the turn (around line 3745). `Key::Ctrl('a')` is Home in the
  composer (around line 7949). An open picker must take these keys first. The routing for an open picker is
  the point to check in the tests.
- The session list's right column shows the message count through `session_row`. `src/store.rs` holds the
  session metadata that carries the directory and the last activity time.
- Picker rows that are not items (headings) have no path through `Picker::index`. Headings need the same
  skip the gap rows have: they are drawn in the window but not selectable.

## Decisions

1. **Footer bar.** When a picker declares row actions, the bottom row of its box is a footer bar with the
   panel fill. The first action sits at the left, the rest at the right in declaration order, as
   `name key`, with the key muted. The bar replaces the title row's action list from `picker-style`. The
   `esc` label stays in the title row. Judgement call: the `esc` key is not repeated in the footer.
2. **Ctrl-key row actions.** Row action keys are Ctrl letters. Letters always type into the filter. The table:
   - Sessions: `ctrl+a` current directory (filter), `ctrl+n` new, `ctrl+d` delete, `ctrl+r` rename.
   - Providers, revoke, add, allow: the same mapping, with the Ctrl letter of the first letter of the action.
     Judgement call: every picker moves to Ctrl keys for consistency. Cost if wrong: one key per action changes.
3. **Ctrl precedence.** An open picker takes `ctrl+d` and `ctrl+r` before the global quit and retry. With no
   picker open, both keep their global meanings. The routing test covers both sides.
4. **Day groups.** Sessions group by the local calendar day of their last activity. The first group is
   `Today`. Earlier groups show the full date, such as `Wed Nov 25 2026`. A heading is a row in the window with
   no item. The window counts headings, so the fixed height from picker-style holds. Moving the highlight skips
   headings. Judgement call: grouping uses last activity, not creation. Cost if wrong: a session sits under a
   different heading.
5. **Right column.** The highlighted session shows its directory, cut from the left. Other rows show nothing
   in the right column. Judgement call: the message count leaves the list. The reference shows no count. Cost
   if wrong: the count is lost from the list; the session's own view still has it.
6. **Current directory.** `ctrl+a` filters the list to sessions whose directory is the current session root,
   and a second `ctrl+a` clears the filter. Judgement call: it is a toggle, not a one-way filter.
7. **New session key.** The reference shows three actions. This design keeps `ctrl+n new` so the action is not
   lost. Judgement call: the footer shows four actions, not three. Cost if wrong: remove the `new` declaration.

## Sequencing

- `picker-style` must be archived before this change is archived. Its requirement "The picker title row carries
  its labels" says the picker draws no key-hint line, and this change reverses that for the footer bar. The
  reversal is written as a MODIFIED requirement after `picker-style` is archived, so the header exists in the
  main spec. Until then, this change's delta covers the row-action and column requirements that are already in
  the main spec.
- `session-goals` and `picker-style` are already merged into local `main`, so this change's base is current.

## Risks

- Risk: Ctrl+D and Ctrl+R are used at the global level. A picker routing bug would quit the app or retry a
  turn. The routing test covers both.
- Risk: some terminals send Ctrl+letter keys differently, such as Ctrl+H for Backspace. Ctrl+A, Ctrl+D, Ctrl+N,
  and Ctrl+R do not collide with that, but the terminal check covers them.
- Assumption: the session metadata holds the directory and last activity. The check in `src/store.rs` is the
  first task.

## Expected bytes

The change adds a footer row, the day headings, and a key table. The size change is measured with
`./scripts/size.sh` after implementation. No fixed figure is claimed here.
