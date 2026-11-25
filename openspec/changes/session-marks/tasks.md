# Tasks: session marks for every session

## 1. Metadata

- [x] 1.1 Test: a session file without `waiting` or `unread` reads both as false, and an older file opens without error.
- [x] 1.2 Test: writing `waiting` and `unread` and reading them back returns the same values.
- [x] 1.3 Implement: the two fields in `Meta` and in `Log::write_meta` in `src/store.rs`, with defaults.

## 2. Saving state

- [ ] 2.1 Test: a question or permission opening sets `waiting`, and answering clears it, both saved. Not yet a UI-level test; the flag rule is tested in `mark_flags`.
- [ ] 2.2 Test: a finished reply in a session not in view sets `unread`, and viewing the session clears it, saved. The rule is in `mark_flags`; the viewing clear happens on the next sync. No UI-level test yet.
- [ ] 2.3 Test: a streamed token does not write the metadata file. Not yet written; `set_marks` writes only on change.
- [x] 2.4 Implement: `Log::set_marks` in `src/store.rs`, `mark_flags` and `App::sync_marks` in `src/tui.rs`, called from the event loop after `poll`.

## 3. Marks

- [ ] 3.1 Test: a closed session with `waiting` set shows `?` in the session picker. Covered by `a_saved_waiting_session_that_is_not_open_gets_a_mark`.
- [ ] 3.2 Test: a closed session with `unread` set shows `•` after a restart. Not yet written; the path is the same as 3.1 with `unread`.
- [ ] 3.3 Test: an open session shows its live state, not the saved flag. Not yet written.
- [x] 3.4 Implement: saved marks read for sessions that are not open, at the end of `session_marks`, with `state_mark`.

## 4. Docs and validation

- [x] 4.1 Update the session marks paragraph in `README.md`.
- [ ] 4.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result so far: 839 unit tests pass; the two `tests/tty.rs` failures predate this change.
