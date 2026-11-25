# Tasks: session marks for every session

## 1. Metadata

- [x] 1.1 Test: a session file without `waiting` or `unread` reads both as false, and an older file opens without error.
- [x] 1.2 Test: writing `waiting` and `unread` and reading them back returns the same values.
- [x] 1.3 Implement: the two fields in `Meta` and in `Log::write_meta` in `src/store.rs`, with defaults.

## 2. Saving state

- [ ] 2.1 Test: a question or permission opening sets `waiting`, and answering clears it, both saved.
- [ ] 2.2 Test: a finished reply in a session not in view sets `unread`, and viewing the session clears it, saved.
- [ ] 2.3 Test: a streamed token does not write the metadata file.
- [ ] 2.4 Implement: one helper in `src/tui.rs` that writes the flags when they change, called from the question, confirm, reply-finished, and view paths.

## 3. Marks

- [ ] 3.1 Test: a closed session with `waiting` set shows `?` in the session picker.
- [ ] 3.2 Test: a closed session with `unread` set shows `•` after a restart.
- [ ] 3.3 Test: an open session shows its live state, not the saved flag.
- [ ] 3.4 Implement: a branch in `session_marks` that reads saved flags for sessions not open.

## 4. Docs and validation

- [ ] 4.1 Update the session marks paragraph in `README.md`.
- [ ] 4.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome.
