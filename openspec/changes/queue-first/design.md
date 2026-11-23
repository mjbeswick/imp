# Design

## Approaches

- **Queue by default, Ctrl+S to steer (chosen).** The common act is the safe
  one; the rarer correction has its own key.
- **Keep steering by default.** Surprises people who type ahead.
- **A different key for steer.** Ctrl+S is free and pairs with Ctrl+Q; both
  reach imp because raw mode turns flow control off.

## Details

`App::queue_mode` starts from `send_mode`. Ctrl+S runs the steer path of
`submit` whatever the mode; in the queue editor it runs `send_queued`, the
path Ctrl+Q used. The rule label becomes `editing n/m · Enter save · ^S
steer|send · ^X del`. A goal still ends on any message, as before.

`/settings` can gain a send mode row later; it is not part of this change.
