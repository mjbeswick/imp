# Proposal

## Why

The agent can start a background job, but once its reply ends nothing wakes
it again, so a 40-minute copy is noticed only at the user's next message. A
job cannot start later, and `job list` says how long a job has run but not how
far it has got.

## What Changes

- `bash` takes `delay_s` (1 s to 24 h), which starts the command as a job
  that waits first; it shows as `starts in …`, and `job kill` cancels it.
- `job check` with `after_s` (30 s to 24 h) wakes the session later: the
  worker runs a turn that opens with a check-in note naming the job's state
  and its latest output. `bash` takes `notify`, which wakes the session when
  that job ends.
- `job list` and check-ins show elapsed time, output size, and the last
  output line, so a copy printing progress shows how far it has got.
- At most four check-ins wait per session. A woken turn is an ordinary turn:
  the same step limit, the same approvals. It counts as a reply in the queue.
- Check-ins run in the chat interface; line mode refuses them.

## Capabilities

### Modified Capabilities

- `tools`: delayed jobs, check-ins, and job progress.
- `inbox`: a check-in wakes its session.

## Impact

- `src/jobs.rs`, `src/agent.rs`, `src/tui.rs`, `src/prompt.rs`, `README.md`.
