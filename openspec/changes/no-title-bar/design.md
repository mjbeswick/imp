# Design

## Approaches

- **Drop the row, move the name to the terminal title (chosen).** Frees a row
  on every screen. The tab or window already has a title, and the status line
  has room for the one value the user acts on, the position.
- **Keep the bar only when several sessions are live.** Keeps the clock and
  the name on screen, but still costs a row whenever the user is juggling
  sessions, which is when the transcript matters most.
- **Fold the name into the status line.** The line already carries the model,
  tokens, cost, goal, queue, and directory; adding a long title crowds them
  and drops content on narrow screens.

## Details

`session_name` stays the single source of the title. `render` compares it with
the last title written and emits `ESC ] 0 ; <name> ESC \` only when it
changes, so a normal frame writes nothing. Controls are stripped and the name
is capped at 80 characters. Before the first message the title is `imp`; after
it, `<name> · imp`.

`show`, `/rename`, and `Ev::Title` all mark the app dirty, so the next frame
carries the new title without a new event.

The layout loses its `title` row. `clock`, `clock_shown`, and `clock_due` go
with it, along with the timeout check in the loop. The position joins `left`
inside `status_line`, so the existing width fallbacks and the directory gap
account for it.

Expected size: a few hundred bytes smaller. Measure with `./scripts/size.sh`
at archive.
