# Design

## Decisions

- **A watchdog over a non-blocking write.** Marking stdout non-blocking would
  change the terminal's open file for the shell too; a pipe to a sleeping
  thread changes nothing outside imp.
- **One byte to a pipe** is async-signal-safe, as `write` is.
- **One second** is enough for a terminal that is reading to take the reset.

## Keyboard

No change.

## Resources

One parked thread and one pipe.
