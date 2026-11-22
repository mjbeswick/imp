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
it and writes nothing to the transcript. While a turn runs the prompt glyph
is `>` for steer and `»` for queue, and the status line says `steer` or
`queue`. When idle, Enter sends and the glyph is `>`. When a turn ends and no queued message is open, the oldest is sent.
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

## `/queue`

`/queue` lists, `pop n` loads into the draft and removes, `delete n`
removes, `clear` empties. It keeps `/stash`'s slot in the command list.

## Order

This change modifies "The interface is keyboard first" from
`tool-row-clarity`'s text, and the command list after it; archive it after
`tool-row-clarity` and before `keys-and-cycling`.

## Size

Expected: 2.5 to 3.5 KB.
