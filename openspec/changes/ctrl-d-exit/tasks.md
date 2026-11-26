# Tasks

## 0. Sequencing

- [x] 0.1 After `modal-shortcuts` is archived, add the picker delta: MODIFIED "Picker rows can carry shortcut actions" and "The modal footer shows shortcut keys", with `ctrl+x` for delete and the Ctrl+D scenarios replaced.

## 1. Tests first

- [x] 1.1 Test: with text in the composer, Ctrl+D exits at once and the draft is discarded. Confirm it fails before the change.
- [ ] 1.2 Test: with a picker open, Ctrl+D exits and runs no row action.
- [ ] 1.3 Test: with a confirmation open, Ctrl+D exits.
- [ ] 1.4 Test: Ctrl+X runs the highlighted session's delete action in the session picker, and Ctrl+D no longer does.

## 2. Implement

- [ ] 2.1 Remove the empty-composer guard from Ctrl+D and handle Ctrl+D before the open picker's keys (`src/tui.rs`).
- [ ] 2.2 Move every row action that used Ctrl+D to Ctrl+X in the row-action table.

## 3. Existing tests and validation

- [ ] 3.1 Update `ctrl_d_keeps_a_draft` and the picker tests that used Ctrl+D for delete.
- [ ] 3.2 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [ ] 3.3 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 3.4 Check in a real terminal that Ctrl+D exits from an empty composer, a draft, and a picker, and record the result in design.md.
