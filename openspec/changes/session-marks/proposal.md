# Proposal: session marks for every session

## Why

The session list marks a session that is waiting for an answer (`?`), has an unread reply (`•`), or
failed (`!`). These marks come only from live workers, so a session that is not open in this window
shows no mark. The unread state is also lost on restart. A user who has several sessions cannot see
which one needs them.

## What Changes

- Each session's metadata records whether it is waiting for an answer and whether it has an unread reply.
- The session list shows the same marks for every session, whether or not it is open.
- A session's state is saved when it changes: a question or permission opens or closes, a reply finishes,
  or the user views the session.
- Existing session files, which lack the new fields, read as not waiting and not unread.

## Out of Scope

- Changing the marks themselves or their meaning. `?`, `•`, `!`, and the streaming spinner keep their
  current meaning.
- Notifications outside imp.
- Syncing state between machines.

## Impact

- `src/store.rs`: two new metadata fields with defaults, read and written with the existing metadata.
- `src/tui.rs`: the mark source reads the saved state for sessions that are not open, and saves it on
  change.
- `src/tui/tests.rs`: tests for marks on closed sessions, marks after restart, and old files.
- `README.md`: the session marks paragraph, which says marks show only for sessions open in this window.
