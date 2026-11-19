# Proposal

## Why

There is no way to try a different direction without losing the current one.
`/new` clears the conversation, and editing the same session overwrites the
history in place. A user who wants to branch from here has to duplicate files by
hand under the state directory.

## What Changes

- Add `/fork`, which copies the running session's transcript and sidecar to a
  new id, names the copy after the source, and attaches the copy to the session.
- Keep the source session untouched and resumable.
- Report both ids: the fork the user is now in and the source they came from.
- Work identically in chat and line mode.
- The slash-command list is updated when this lands. `image-paste` and
  `cli-polish` also touch that list, so the count follows the merge order.

## Capabilities

### Modified Capabilities

- `session`: a session can be copied into a new fork that continues from the
  same history without altering the original.

## Impact

- `src/store.rs`: a `Log::fork` that copies transcript and sidecar.
- `src/agent.rs`: `Session::fork`, attaching the copy and logging the source.
- `src/tui.rs` and `src/main.rs`: the command in both front-ends.
- `src/help.rs` and `README.md`.
