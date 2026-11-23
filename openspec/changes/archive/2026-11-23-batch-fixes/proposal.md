# Proposal

## Why

A review of today's nine changes found gaps:

- A toast set the loop's wait to its remaining time, up to three seconds,
  even while a turn streamed, stalling tokens and the spinner.
- `/new` and resuming built sessions from the startup config, so a value
  `/reload` changed, such as `allow_sudo`, did not reach them.
- Tests read and wrote the user's real prompt history, as they did the theme.
- `localtime_r` need not read `TZ` without `tzset`.
- `/effort` on an OpenAI-compatible provider does nothing while thinking is
  off, and said nothing.
- A paste into a picker replayed its letters, and some letters act on rows.

## What Changes

- The toast wait is the smaller of its time left and the usual wait.
- New and resumed sessions take the chat's current config.
- Tests leave the history file alone.
- The clock calls `tzset` once.
- The effort toast adds `applies with thinking on` where that holds.
- Pastes are ignored while a picker is open.

## Keyboard and resources

No keys change.
