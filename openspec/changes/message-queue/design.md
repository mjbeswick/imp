# Design

## Approaches

- **Enter sends; Ctrl+Q toggles steer and queue (chosen).** One send key,
  a key that works in every terminal, and Alt+Enter stays a newline for
  terminals without the kitty protocol. The cost is a mode to forget, so it
  shows where the user types.
- **Enter steers, Alt+Enter queues.** Takes the most portable newline key,
  and macOS terminals send Option+Enter as Enter by default. Rejected.
- **Ctrl+Enter to queue.** Most terminals, tmux, and the Hackberry console
  send it as Enter. Rejected.
- **Keep `/stash` beside the queue.** Two lists for one job. Rejected.

## Queue

`App.queue: Vec<String>` per session, kept with the draft on a switch.
`App.queue_mode: bool` per session, default off (steer). Enter during a
turn steers, or appends to the queue when `queue_mode` is on; Ctrl+Q flips
it and writes nothing to the transcript. While a turn runs the rule above
the composer carries `steer · ^Q queue` or `queue · ^Q steer`, set into it as
the picker sets its title, in the rule's own colour, so yolo's red holds;
the glyph is `>` for steer and a muted `»` for queue, which still shows when
the screen is too short for the rule. The status line does not repeat it.
While a queued message is open the rule reads `editing queued n/m · Enter
save · ↑↓ move`. When idle, Enter sends, the glyph is `>`, and the rule is
plain. When a turn ends, no queued message is open, and the user did not stop it,
the oldest is sent; after a stop the queue waits, since starting the next
message would undo the stop.
Rows above the composer: `⋯ <text>`, one row each, at most three, then
`+N more`; the open one is highlighted.

↑ on an empty draft, or from the draft's top row with no history position,
opens the newest queued message; further ↑ walks to older ones, then into
history. ↓ walks back and finally to an empty draft. Leaving an edited
message saves it, as Enter does. Saving it empty removes it.

While a goal runs, a queued or steered message is sent at once, as today,
so the goal ends with "you took over".

## Steering and pausing

`Session.steer: Arc<Mutex<Vec<String>>>` and `Session.hold: Arc<AtomicBool>`,
shared with the chat. Before each model request in a turn, the agent waits
while `hold` is set, checking cancel, then drains `steer` into user messages.
The chat sets `hold` while a queued message is open. A reply already
streaming or a tool already running finishes first; the status line shows
`paused · editing queue`.

The chat draws a steered message as a user row marked `(steered)`. One
steered while the model writes its final reply is never read by that turn;
when the turn ends the chat sends what is left as the next turn, ahead of the
queue, or after a stop puts it at the front of the queue.

## `/queue`

`/queue` lists, `pop n` loads into the draft and removes, `delete n`
removes, `clear` empties. It keeps `/stash`'s slot in the command list.

## Order

This change modifies "The interface is keyboard first" from
`tool-row-clarity`'s text, and the command list after it; archive it after
`tool-row-clarity` and before `keys-and-cycling`.

## Size

Expected: 2.5 to 3.5 KB. Measured: `__text` +3832, `__cstring` +208.
