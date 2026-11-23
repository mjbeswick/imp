# Design

## Approaches

- **Queue on Enter, steer on Ctrl+S, no mode (chosen).** Each key does one
  thing; nothing to remember or check.
- **Keep a mode and flip the default.** Leaves the hidden state.

## Details

`App::queue_mode` and its field in each pane go, as do the Ctrl+Q handler,
the `steer or queue` palette row and its keys card line. Ctrl+S runs the
steer path of `submit`; in the queue editor it runs `send_queued`. The rule
label becomes `editing n/m · Enter save · ^S steer|send · ^X del`. A goal
still ends on any message, as before. People who liked steering on Enter use
Ctrl+S.
