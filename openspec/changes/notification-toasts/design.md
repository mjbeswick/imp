# Design

## Context

- `apply_to` in `src/tui.rs` (around line 2601) applies an event to a
  background pane. It already rings the bell after the swap back.
- Job completions are emitted as `Ui::Info` at three sites in `src/agent.rs`
  (around lines 1019, 1113 and 1463). The same text goes to the transcript and,
  at two of the sites, to the model.
- Wakes arrive as `Ev::Woke(what)` (around line 2979).
- `toast()` writes the single status-line slot shown for three seconds.
- The session name is shared logic with the title bar: `title_bar` (around
  line 2851) picks the stored title, then the cut first message, then `imp`.

## Decision

- Add `Ui::Notice(String)` and `Ev::Notice(String)`. The three job sites emit
  `Notice` with the same text. In the chat, `Notice` is applied exactly as
  `Info` is, so the transcript does not change. A background pane turns it
  into a toast.
- In `apply_to`, when the pane is not the one in view, build the toast text
  while the pane is swapped in: position from the pane index and count, name
  from the shared session-name helper, and the note. Set the toast after the
  swap back, so it appears on the status line of the session in view.
- Notes: a turn that finished is `replied`; a failed turn is `error: …`; a
  confirmation or question is `needs an answer`; a wake is `⏰ …`; a job is the
  notice text. The status line truncates long text as it already does.
- Line mode prints a `Notice` as a plain line.

## Risks

- A confirmation in a background session can be answered later, so
  "needs an answer" can stay visible only for three seconds. The bell and the
  queue (`Ctrl+O`) remain the way to find it.
