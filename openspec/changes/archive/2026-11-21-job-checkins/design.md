# Design

## Decisions

- **A delay is a `sleep` in the job's script.** The job exists from the
  start, so `job kill` cancels it and session end stops it, with no second
  timer. `Job` records when it starts for the status line.
- **Check-ins live in the session,** as `(due, job)` pairs. `Session::due`
  returns the next note when a check-in is due or a job the agent asked to be
  told about has ended; `Session::wake` runs a turn from it.
- **The worker waits with a one-second timeout** instead of blocking, and
  asks the session for a due note when no command arrived. A busy worker
  checks after its turn ends.
- **The note is a user-role message marked as from imp,** since some
  providers need a user message to start a turn. It says it is not from the
  user.
- **`Ev::Woke`** tells the interface a turn began, so it shows the note,
  marks the session working, and treats the end like any reply.
- **Progress from the log file:** its size, and its last line split on `\r`
  and `\n`, since progress meters redraw with `\r`.
- **Line mode refuses `check`:** it blocks on input and has no worker to
  wake.

## Keyboard

No change.

## Resources

One timer check a second per live session; no new process.
