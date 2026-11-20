# Design

## Shape

- `App`'s per-session fields always hold the session in view. Sessions out
  of view are `Pane`s holding the same fields; switching swaps them, and an
  event for a pane out of view is applied by swapping it in, calling `apply`,
  and swapping it back, so one code path serves both and no redraw happens.
- Each session has its own worker from `spawn_worker`, with its own command
  sender and event receiver; the main loop polls every receiver. `Ev` needs
  no pane id.
- A new session is built by the current worker, which owns the system prompt,
  roster, and memory, and sent back as `Ev::Spawn`. It gets its own mode
  handle, so `/mode` stays per session, its own cancel flag, so Esc stops one
  session, and its own job directory, so job logs never collide.
- Answering from the inbox sends on that pane's pending reply; the agent loop
  is unchanged.
- The confirmation deadline moves from the worker to the UI: the worker waits
  without a limit, and the UI answers `n`, cancels the turn, and says why when
  a prompt has been in view for 60 seconds.
- Buffers keep separate address spaces per session; a process-wide byte count
  makes each session evict its own least recently used buffers when the total
  passes the cap.

## Order

Archive `permission-mode` and `approval-always` first. This change's deltas
are written against their results: `/mode` in the command list and `a` in
confirmations.

## Keyboard

Every action is one chord or one letter: Ctrl+O inbox ("overview"), Ctrl+N and
Ctrl+P next and previous, Ctrl+E expand. Inside the inbox, plain letters and
arrows. No Tab, Shift+Tab, function keys, or modifier-hold detection, which
the Hackberry Pi Zero's keyboard and the Linux console cannot rely on. `?` on
an empty composer shows the card when the keys are forgotten.

## Resources

- Target: Pi Zero 2 W, 512 MB. Four live sessions at most. An idle session is
  one parked thread, its transcript, and its buffers; the shared 16 MB buffer
  cap keeps the total bounded instead of growing per session.
- Only the pane in view is drawn; background events update state and the
  status count without a redraw of their transcript.
- Expected binary growth: 10 to 15 KB, mostly the pane split and inbox
  rendering. Measured bytes go in the archive commit.

## Risks

- The pane split touches most of `tui.rs`. It lands first as a refactor with
  one pane and no behaviour change, before any second session exists.
