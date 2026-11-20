# Design

## Decisions

- **One cache on the app, one slot per entry.** A slot holds a hash of the
  width, the markdown flag, the blank row before a turn, the role, and the
  text, plus the rows. A changed hash renders that entry again.
- **Hash, not a copy.** Keeping a copy of every text would double the
  transcript's memory. Hashing is linear but far cheaper than rendering.
- **Not swapped with the session.** Switching sessions misses the cache once
  and refills it. Correct, and simpler than a cache per pane.
- **Rows are borrowed.** The frame paints the visible rows from the cache
  without cloning them.

## Keyboard

No change.

## Resources

One `u64` and the rendered rows per entry, roughly the transcript's size again
in rows already wrapped.
