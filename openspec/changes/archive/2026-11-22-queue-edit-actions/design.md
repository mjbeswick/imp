# Design

## Approaches

- **Two keys on the open message (chosen).** `Ctrl+Q` already means
  steer-versus-queue, and `Ctrl+X` is unused, so the editor adds no new
  vocabulary.
- **`Ctrl+D` deletes.** Rejected: it is quit outside the editor and delete
  inside it, and an emptied edit makes the two meanings meet.
- **A per-message queue/steer flag kept in the list.** Rejected: steering only
  means something while a turn runs, so the flag would be read once, at the
  moment the key is pressed.
- **A separate queue screen.** Rejected: the composer already is the editor,
  and a screen would duplicate the list above it.

## Behaviour

`Ctrl+Q` takes the composer's text, drops that entry from the queue, clears
the composer, closes the editor, and then follows the submit path: a
`(steered)` row and the steer buffer while a turn runs, `start_turn`
otherwise. An empty edit at that moment deletes, as Enter does.

`Ctrl+X` removes the entry, then opens the entry that moved into its place,
or the last one when it was the end. An empty queue closes the editor.

`Ctrl+C` closes the editor without writing: the queued text stays as it was,
and it never quits while a message is open.

## Releasing the queue

Leaving the editor has six exits: Enter, ↑ into history, ↓ past the end,
`Ctrl+C`, `Ctrl+X`, and `Ctrl+Q`. Each one calls one helper that drops
`queue_at`, clears the hold, and calls `release_queue`. That fixes the stall
where ↑ left `queue_due` set forever.

`open_queued` and `save_queued` clamp or abandon a stale index, so a queue
that shrank under the editor can never panic or draw `2/3` over two items.

## Size

Expected: under 2 KB.
