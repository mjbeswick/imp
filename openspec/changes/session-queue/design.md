# Design

## Decisions

- **One new field.** Each session keeps `since`, set when an approval or
  question opens and when a reply finishes out of view. `touched` changes on
  every event and `pending_since` resets on `show()` for the deadline, so
  neither can order the queue.
- **Queue = waiting sessions out of view, oldest `since` first.** Viewing a
  session clears `replied`, so a read reply leaves the queue by itself; an
  approval in view gets `since = now` when the user moves on, which sends it
  to the back.
- **Hand-off detection in `handle_key`.** A key that turns an open approval or
  question into none, or starts a turn, is a hand-off. Ctrl+C is not.
- **Note in the destination's transcript** as an info line, like other
  command replies; no transient status state.
- **Deleted, not hidden:** the inbox renderer, `PickKind::Inbox`, and the
  number keys go, which should shrink the binary.

## Keyboard

Ctrl+O is the only key. Repeated presses walk the queue.

## Resources

One `Instant` per session. Expected to save 1–3 KB against the cards.
