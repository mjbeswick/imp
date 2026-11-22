# Design

## Approaches

- **Enter steers, Alt+Enter queues (chosen).** The user's choice: typing
  during a turn is usually a correction.
- **Ctrl+Enter to queue.** Most terminals, tmux, and the Hackberry console
  send it as Enter. Rejected.
- **Ctrl+Q alone.** Works everywhere but is not where hands go. Kept as the
  fallback shown beside Alt+Enter.
- **Keep `/stash` beside the queue.** Two lists for one job. Rejected.

## Queue

`App.queue: Vec<String>` per session, kept with the draft on a switch.
Enter during a turn steers; Alt+Enter or Ctrl+Q appends to the queue at any
time. When a turn ends and no queued message is open, the oldest is sent.
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
