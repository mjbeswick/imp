# Tasks

## 0. Sequencing

- [ ] 0.1 Archive `picker-style` before archiving this change. Then add the MODIFIED requirement for "The picker title row carries its labels" so it no longer forbids the footer bar.

## 1. Tests first

- [x] 1.1 Test: a picker with row actions draws a footer bar with the first action at the left and the rest at the right, in declaration order, and draws no other key-hint text. Confirm it fails before the change.
- [x] 1.2 Test: a picker with no row actions draws no footer bar and keeps its height.
- [x] 1.3 Test: with a non-empty filter, every letter types into the filter; Ctrl+R renames the highlighted session when the picker is open.
- [x] 1.4 Test: with a picker open, Ctrl+D runs the delete action and does not quit; with no picker open and the composer empty, Ctrl+D still quits.
- [ ] 1.5 Test: the session picker groups sessions under `Today` and earlier full dates; the highlight skips headings; a heading takes one window row.
- [ ] 1.6 Test: the highlighted session's directory is in the right column, cut from the left; other rows have no right column.
- [ ] 1.7 Test: Ctrl+A filters to the current directory and a second Ctrl+A clears it, with the typed filter unchanged.

## 2. Row actions

- [x] 2.1 Implement the Ctrl-key row action table in `Picker` and its key routing, so an open picker takes Ctrl+D and Ctrl+R before the global keys (`src/tui.rs`).
- [x] 2.2 Move the session, provider, revoke, add, and allow actions to their Ctrl keys (design.md table).

## 3. Footer bar

- [x] 3.1 Draw the footer bar at the bottom of the box with the panel fill, the first action at the left and the rest at the right, and remove the footer hint text (`src/tui.rs`).

## 4. Sessions

- [ ] 4.1 Group sessions by the local day of last activity, with headings as non-selectable rows inside the window (`session_picker`, `src/store.rs`).
- [ ] 4.2 Show the highlighted session's directory in the right column and drop the message count from the list.
- [ ] 4.3 Implement Ctrl+A as the current-directory filter toggle.

## 5. Existing tests and validation

- [ ] 5.1 Update picker tests that assert single-letter actions, the footer hint text, and the message count in the session list.
- [ ] 5.2 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [ ] 5.3 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 5.4 Check the footer bar and the day groups in a real terminal against the reference, and record the result in design.md.
