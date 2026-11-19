# Design

## Approaches

1. **Render on change only, throttle the spinner.** Delete the unconditional
   timer clause and gate the spinner on elapsed time. Chosen: the loop already
   knows when something happened, and 10 fps is enough for a spinner.
2. **Keep the timer, lower it to 500 ms.** Still writes 2 frames a second of the
   same content, so the flicker only slows down.
3. **Double-buffer in the app.** A terminal has no back buffer; the flicker comes
   from the emulator's repaint, so this cannot be fixed on our side.

## Decisions

- The render condition becomes `redraw || app.dirty`. Every path that matters
  already sets one of them: keys, events, resize ticks, and the spinner.
- The spinner advances only when at least 100 ms passed since the last frame, so
  a streaming turn draws ten frames a second instead of twenty.
- `last_redraw` keeps its role as the spinner clock.
- Measured, not assumed: idle bytes written should fall from about 4.1 KB/s to
  zero, checked with `/proc/<pid>/io` on the Pi.
