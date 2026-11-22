# Design

## Approaches

- **No timeout (chosen).** A prompt waits for days under tmux or dtach, and
  the inbox already queues sessions that wait. Nothing is lost by waiting.
- **An optional `confirm_timeout` setting, off by default.** Keeps code and a
  setting for a case nobody asked for. Rejected; it can come back for goal
  rounds and check-ins if an unattended prompt ever blocks one.
- **Keep the 60 s deadline.** Loses turns and pressures approval. Rejected.

## Default answer

Yes stays the default, as the user chose: `y` and Enter approve, `n` and Esc
decline. A key typed just before a prompt opens can still approve it; a short
key guard would close that gap if it ever bites.

## Code

`CONFIRM_WAIT`, `expire`, `seconds_left`, and `pending_since` go, with the
pane swap of `pending_since` and the restart of the deadline in `show`. The
confirmation row drops its right-edge seconds. `loop_wait` loses its
one-second branch for an open prompt. `Ev::Stop` stays: other paths use it.

The test that a countdown tick writes one row becomes a test that a changed
status line writes one row.

## Size

Expected: a few hundred bytes smaller. Measured: `__text` -596, `__cstring` -59.
