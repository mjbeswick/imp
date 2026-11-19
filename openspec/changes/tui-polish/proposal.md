# Proposal

## Why

The interface works but looks unfinished. A resumed session opens on a blank
screen with `0 tok`, though its history is loaded. The empty start screen gives
no sign of where you are. The transcript, composer, and status line run together
with nothing between them. The picker crams its title, filter, and position into
one row, and the session list loses its columns when a title is long.

## What Changes

- Resume draws the restored conversation and its token estimate, from `-r`,
  `-c`, and the session picker. The picker title stops saying "after restart".
- A dim rule separates the transcript from the composer. Each user turn after
  the first starts after one blank row.
- An empty transcript shows one dim welcome row: name, directory, and the
  `/` hint.
- The status line keeps the model first and adds the session directory,
  right-aligned, dropped first when the row is narrow.
- Tool durations are dim and right-aligned: milliseconds under one second,
  seconds with one decimal above.
- Pending prompts paint in the accent role behind a `▌` bar, and the rule above
  them takes the accent too.
- The picker gets rounded corners, its title and position in the top edge, its
  key hints in the bottom edge, and the filter on its own row. Width caps at 72
  columns.
- Picker rows with a right column keep it whole and cut the label instead.
  Sessions show the title on the left and age and message count on the right.

## Out of Scope

- `/help` layout: the open `help-format` change covers it.
- An ASCII fallback for box characters. The current picker already assumes
  UTF-8.
- Git branch in the status line. It costs a process per frame or a cache.

## Capabilities

### Modified Capabilities

- `tui`: chat column layout, status directory, tool durations, prompt accent.
- `picker`: overlay frame and aligned columns.
- `session`: resume draws the restored transcript.

## Impact

- `src/tui.rs`: frame, composer, status line, overlay, picker rows, resume
  event.
- `src/agent.rs`: build display entries from restored history.
- `src/store.rs`: split the session row into title and detail.
- `README.md` if the picker sample changes.
