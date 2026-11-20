# Design

## Decisions

- **One command, three values.** `/mode` reuses `value_picker()` and
  `mark_current()`, so it costs a list of three rows and a match arm. `/yolo`
  goes rather than staying as an alias: one way to do each thing.
- **The old objection.** The first design rejected `/mode` because a quick
  permission change invites flipping guardrails mid-session. `/yolo` already
  allowed that; `/mode` adds only the safer direction, `strict`, and a visible
  current value.
- **Session only.** The launch mode comes from `--permissions`,
  `IMP_PERMISSIONS`, or the config; `/mode` changes `live_permissions` only.
  The transcript line names the new mode, for example
  `mode: yolo · deny list still applies · path jail off`.
- **Applied at once.** As `/yolo` was, the new mode is applied to the shared
  mode before the worker sees the command, so a confirmation in the running
  turn honours it.

## Keyboard

The target is the Hackberry Pi Zero's compact keyboard. `/mode y` and Enter is
seven keys, close to `/yolo`'s six. The picker needs only Up, Down, Enter, and
Esc; no chords or function keys.

## Resources

No new state, threads, or allocation beyond three static rows. Expected binary
growth is under 1 KB against 3.4 KB of headroom.
