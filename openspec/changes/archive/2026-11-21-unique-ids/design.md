# Design

## Decisions

- **Exclusive create.** `create_new` on the sidecar is atomic, so two
  processes racing for the same second still get different ids. Checking
  `exists()` first would leave that race open.
- **Same suffix as fork.** `<id>-1`, `<id>-2`. Ids stay sortable by time.
- **`symlink_metadata` in the jail walk.** A link counts as present even when
  its target is missing; canonicalising it then fails, and the write is
  refused with the path named.

## Keyboard

No change.

## Resources

One extra open per session start.
