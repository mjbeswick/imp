# Tasks: command palette

## 1. Palette

- [ ] 1.1 Test: Ctrl+P opens a `Command` picker with one row per command in `COMMANDS`, no prefix, and an empty filter.
- [ ] 1.2 Test: typing `qu` in the palette filters the rows to commands containing `qu`.
- [ ] 1.3 Test: Enter on `/quit` in the palette quits the session.
- [ ] 1.4 Test: Esc closes the palette and leaves the composer text unchanged.
- [ ] 1.5 Test: `/quit` typed in the composer still quits, with the palette code present.
- [ ] 1.6 Implement: the `Key::Ctrl('p')` handler in `src/tui.rs` that opens the command picker with no prefix.

## 2. Docs and validation

- [ ] 2.1 List Ctrl+P in `src/help.rs` (the keys help) and in `README.md`.
- [ ] 2.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome.
