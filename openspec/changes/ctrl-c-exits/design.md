# Design

## Approaches

1. **A latch on the app.** `cancel_requested` is set on the first press during a
   turn, cleared when the turn ends, and the second press quits regardless of
   streaming state. Chosen: one boolean, no timer, and it survives the gap where
   the worker is still unwinding.
2. **A two second timer.** The old guard. It fails exactly when you need it,
   because the turn can outlive the window.
3. **Treat any press during a turn as quit once cancelled.** Loses the ability to
   cancel twice in a row after a mis-press.

## Decisions

- The latch resets on `Done` and on `Stop`, so a cancelled turn does not leave
  the next press one keystroke from quitting.
- The first press still cancels through the existing `cancel` flag, so tools see
  it the same way as before.
- The status row carries the hint only while the latch is set. It returns to
  model and cost once the turn ends.
- The quit guard scenario in the spec is replaced, not extended. A guard that
  does not exist should not be promised.
