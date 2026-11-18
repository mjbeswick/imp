# Proposal

## Why

Two small legibility gaps in the transcript and the composer.

- Inline code is painted with the thinking grey, so `` `bash` `` fades into the
  sentence around it. A live reply read "could not bash it into existence" with
  the tool name almost invisible.
- `!` already runs a local command, but nothing in the interface says so: the
  prompt still reads `>` and the status row still reports model and cost while
  the draft is headed for the shell.

## What Changes

- Inline code renders bold in the tool colour; fenced blocks and quotes keep the
  dim style.
- The composer prompt becomes `$ ` when the draft starts with `!`.
- The status row reads `shell · local` while such a draft is open.
- The key list names `!` as the shell prefix.

## Capabilities

### Modified Capabilities

- `tui`: inline code is distinct from dim text, and a shell draft is visible.

## Impact

- `src/tui.rs`: span painting, composer prefix, status line.
- `src/render.rs`: fenced blocks stop carrying the code style.
- `src/help.rs`: one key line.
- Spec deltas under `tui`.
