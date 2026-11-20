# Design

## Decisions

- **Full screen, status kept.** Four cards of two to four rows fit the
  Hackberry's screen; the status line keeps the `waiting` count and mode in
  sight. An overlay over a transcript the user is not reading buys nothing.
- **Picker kept as state.** The `Picker` still holds the order, highlight,
  and per-row actions; only drawing and keys change. `desc` keeps the state
  first so ordering and tests stay as they are.
- **Detail built at draw time** from the pane: pending command or question,
  the tool row, or the last reply lines. Nothing new is stored.
- **Chips shared** with the confirmation row through one helper.
- **Scroll to the highlight** when cards outgrow the screen.
- **Number keys** match the status line's session numbers, not card order,
  so `2` is always the session the status line calls `2/3`.

## Keyboard

Up and Down move, Enter opens, `1`–`4` jump, `y n s a` answer, `x` closes,
Esc or Ctrl+O backs out. Single keys only.

## Resources

No new state; the cards are rebuilt per frame from at most four panes.
Expected cost about 3 KB of binary.
