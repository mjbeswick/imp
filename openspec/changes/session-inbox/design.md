# Design

## Shape

- `App` keeps what belongs to the screen: palette, size, overlay, inbox state,
  and a `Vec<Pane>`. A `Pane` holds what belongs to one session: transcript
  lines, composer draft, scroll, pending prompt, state, title, and its
  command sender.
- Each pane has its own worker thread running a `Session`, as today. Workers
  send `(pane id, Ev)` on one shared event channel, so the event loop stays
  single-threaded and draws only the pane in view.
- A confirmation already blocks its worker on a reply channel, so answering
  from the inbox sends on that pane's reply channel; nothing new is needed in
  the agent loop.
- The deadline counts only while the pane is in view: the pending prompt
  stores when it was first shown, not when it arrived.
- The buffer store moves behind a shared handle with per-session address
  spaces, so limits apply to the sum.

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
