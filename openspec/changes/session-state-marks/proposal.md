# Proposal

## Why

The sessions modal is empty when every saved session is blank. `/new` writes a
session's metadata at once, but its transcript appears only after the first
message, and the list skips sessions without a transcript. The modal also
shows nothing about what each session is doing, so a session that is running,
waiting for an answer, or has failed looks the same as an idle one.

## What Changes

- The sessions modal lists every saved session, including blank ones, shown as
  `(untitled)` with `0 msgs`.
- Each row starts with a mark: a spinner while the session runs, `?` while it
  waits for an answer or confirmation, `!` when its last turn failed, and `•`
  when it finished and has not been viewed. An idle row has no mark.
- Marks apply to sessions open in this window. A closed session shows none.
- Search results carry the same marks.

## Keyboard and resources

No keys. The marks reuse the spinner that already animates the modal.
