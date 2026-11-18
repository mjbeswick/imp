# Design

## Approaches

1. **Ctrl+C always exits.** One guard at the top of `handle_key`, before the
   prompt dispatch and the menu. Chosen: one meaning, no state, and the key does
   what the terminal does everywhere else.
2. **Cancel then exit.** What exists now. Two presses during a turn, and a
   refusal on a confirmation, which is what reads as broken.
3. **Exit only when idle, refuse otherwise.** Leaves a busy session with no exit
   key at all.

## Decisions

- The guard runs before `self.pending.take()` and before the menu handling, so no
  state can swallow the key.
- The turn dies with the process. Cancelling mid-answer is Esc's job.
- The latch, its reset points, and the `cancelling · ctrl+c again to exit` line
  are deleted, not left dormant.
- Tests cover the four states that matter: idle, streaming, confirmation open,
  picker open.
