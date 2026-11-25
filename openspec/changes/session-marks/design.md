# Design: session marks for every session

## Context

- Each session has a metadata file `<id>.json`, written by `Log::write_meta` (`src/store.rs`, around line
  517). `Meta` (`src/store.rs`, around line 400) holds id, title, cwd, model, messages and updated.
- The marks come from `session_marks` (`src/tui.rs`, around line 6473). It reads the current worker and
  each open pane, with `state_mark` (`src/tui.rs`, around line 1563) choosing the mark. The
  `waits()` method (`src/tui.rs`, around line 1989) reflects `replied` and a pending confirm or question.
- `replied` is a field on the app and on each pane. It is not written anywhere, so it is lost on restart.

## Decisions

- **Two fields in `Meta`.** `waiting: bool` (a confirm or question is pending) and `unread: bool` (a reply
  finished and has not been viewed). Both are serde-style optional fields in the JSON file, defaulting to
  false, so existing files need no migration.
- **Saved on change.** The worker event handling and the question and confirm handling call one helper
  that writes the two flags to the session's metadata. Viewing a session clears `unread`.
- **One mark source.** `session_marks` keeps its live-worker branch for open sessions, and adds a branch
  that reads the saved flags for every other session in the list. Live state wins when both exist, so an
  open session shows its current state.
- **Failed is not saved.** `!` is a state of the live turn. It is shown only for open sessions, as now.

## Risks

- A crash between a state change and its write can leave a stale flag. Accepted: the mark is a hint, and
  the next event rewrites it.
- Writing metadata on every streamed event would be costly. The flags are written only when they change,
  not on every token.

## Assumptions

- Viewing a session is the moment it counts as read. It is cleared when the session becomes the view, not
  when a reply finishes in another pane.
