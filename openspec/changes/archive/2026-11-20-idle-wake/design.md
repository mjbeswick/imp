# Design

## Decisions

- **Adaptive timeout, not one merged channel.** Each session has its own event
  channel. Merging them is a larger change; an idle worker sends nothing, so a
  longer wait when nothing is active loses no events.
- **Two-second grace.** A command such as `/provider` or `/compact` answers
  through the worker; staying fast for two seconds after any key or event
  covers it, and streaming keeps it fast after that.
- **SIGWINCH through a pipe.** The handler writes one byte, which is
  async-signal-safe; a thread turns it into an empty wake on the key channel.
- **Five-second ceiling.** A safety net for anything missed.

## Keyboard

No change.

## Resources

About 12 wakes a minute when idle instead of 1,200.
