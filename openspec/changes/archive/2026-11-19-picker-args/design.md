# Design

## Approaches

1. **Extend the existing picker and browser.** Chosen: one overlay, one key
   path, and the value lists already have builders.
2. **A second argument menu.** Duplicates the picker and splits the cursor
   rules.
3. **Autocomplete in the composer only.** Keeps the small inline menu that this
   change removed, and cannot show a cursor in a floating filter.

## Decisions

- The directory browser keeps the session root as its starting point. `Left`
  climbs to the parent until `/`; `root` is no longer a lower bound for `Dir`.
  The image browser keeps its root clamp so attachment stays inside the
  session. Selecting an ancestor on Tab sends the existing `/cd` path, so the
  jail moves with the new root.
- `Picker` gains `prefix: String`. For the command list it is `/`; for a value
  list it is `/agent ` or `/agent rev` minus the filter. Typing and Backspace
  update `prefix + filter` in the composer, so the visible line and the filter
  never disagree.
- `sync_menu` splits the input at the first space. No space means the command
  list. A known command word means the value list, built by the same function
  the bare command uses: agents, models, themes, verbosity levels, provider
  names for `use`/`edit`/`delete`, and the directory or image browser when the
  remainder is empty.
- Free-text arguments stay in the composer: `/stash`, `/rename`, `/provider
  add`, and a non-empty `/cd` or `/image` path.
- `PickItem` gains an optional description. The command list fills it from
  `COMMANDS`. The overlay pads the label to the longest visible name plus two
  spaces, never narrower than twelve columns, then writes the description and
  clips the combined row to the interior width. Other pickers leave the field
  empty and render as they do now.
- Filtering stays on the label so a description word cannot pull in an
  unrelated command.
- The overlay draws the real cursor on the filter row with absolute addressing
  and `\x1b[?25h`; every frame starts hidden, and closing the overlay leaves
  the composer cursor alone. If the box does not fit, no overlay and no cursor
  change.
- Tests cover the climb above the root, the argument picker with its prefix,
  free-text staying put, and the cursor sequence in the frame.

## Tests

- `Left` at the root lists the parent; `Left` at `/` is a no-op.
- `/theme ` opens the picker with the theme list; typing narrows it and the
  composer reads `/theme gru`.
- `/stash note` sends no picker.
- The overlay frame contains the cursor placement and visibility sequence.
- Command rows align their descriptions and keep the name whole in a narrow
  box.
