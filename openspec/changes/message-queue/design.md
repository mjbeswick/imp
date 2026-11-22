# Design

## Approaches

- **Enter queues; Ctrl+X steers (chosen).** A queued message can still be
  edited, and nothing interrupts the model by accident.
- **Enter steers; a key queues.** Faster to correct a turn going wrong, but
  every message typed during a turn changes the turn. Rejected; the user
  leaned to queueing.
- **Ctrl+Enter for steering.** Most terminals, tmux, and the Hackberry console
  cannot tell it from Enter. Rejected.

## Queue

`App.queued: Vec<String>` and `App.queue_paused: bool`, per session like the
draft. `submit` during a turn pushes to the queue instead of sending. On
`Ev::Done`, when not paused, the oldest is sent as a normal prompt. Rows
above the composer: `⋯ <text>`, cut to one row each, at most three, then
`+N more`.

Up on an empty draft with no history position and a non-empty queue pops
the newest into the draft and sets `queue_paused`. `submit` clears the
pause; during a turn the text goes back to the end of the queue, otherwise it
is sent and the rest follow as turns end.

While a goal runs, a queued message is sent at once, as today, so the goal
ends with "you took over".

## Steering

`Session.steer: Arc<Mutex<Vec<String>>>`, shared with the chat. Before each
model request in a turn, the agent drains it into user messages. The chat
draws the text as a user row marked `(steered)`. Ctrl+X with no turn
running is Enter.

## Stash

Ctrl+S with a draft runs `/stash <draft>` and clears the draft; on an empty
draft it pops the newest stash, as `/stash pop` does.

## Size

Expected: 2 to 3 KB.
