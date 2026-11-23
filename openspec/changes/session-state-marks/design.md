# Design

## Context

- `Log::list` in `src/store.rs` (around line 571) includes a session only when
  `{id}.jsonl` exists, so a blank session is skipped. `Log::open` requires the
  transcript too, and `Log::latest` relies on `list`.
- `session_picker` (around line 3918) and `search_picker` (around line 3941)
  build the rows. Both call `spin_rows`, which puts the spinner in front of the
  description of a running session.
- Each open pane owns a worker whose shared slot holds its session id, read by
  `working_ids` (around line 5762).
- `Pane::waits` (around line 1909) already reports a session waiting on the
  user. `Pane::replied` marks a finished reply not yet viewed.
- A failed turn is `Ev::Done(Some(error))`. No state keeps it today.

## Decision

- Add `Log::listing(dir)`: the same as `list`, but a session counts when its
  metadata file exists, so the modal can show blank sessions. `list` and
  `latest` keep their behaviour, so `--list` and resume are unchanged.
- Add a `failed` flag to `Pane` and `App`, set by `Ev::Done(Some(_))` and
  cleared by `Ev::Done(None)`. It travels with the pane through the existing
  swap.
- Add `session_marks()`: for each open session id, one mark by priority:
  waiting (`?`), running (spinner frame), failed (`!`), unseen reply (`•`). The
  mark is written in front of the description by one helper that replaces
  `spin_rows` in both pickers.

## Risks

- The modal grows with every `/new`. Blank sessions stay until deleted with
  `d`, which the modal already offers.
