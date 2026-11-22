# Proposal

## Why

There is no way to see what imp changed in a session short of scrolling back
through every edit, and no quick `git diff` from the chat.

## What Changes

- `/diff` shows the net change to every file imp's `edit` and `write` tools
  touched this session: a summary row per file with `+N −M`, then the diffs,
  200 rows at most with a count of the rest, in the diff colours.
- `/diff git [args]` runs `git diff` with those arguments in the session
  directory and shows it the same way.
- Neither is sent to the model.
- The summary says that changes made by shell commands are not tracked and
  points to `/diff git`.

## Keyboard and resources

One more command, taking the list to 25. The first version of each touched
file is kept in memory; files over 1 MB or binary are listed, not diffed.

## Capabilities

### Modified Capabilities

- `tui`: the `/diff` command.
