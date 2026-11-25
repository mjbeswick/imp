# Tasks: command palette

## 1. Palette

- [x] 1.1 Test: Ctrl+P opens a `Command` picker with one row per command in `COMMANDS`, no prefix, and an empty filter.
- [x] 1.2 Test: typing `qu` in the palette filters the rows to commands containing `qu`.
- [x] 1.3 Test: Enter on `/quit` in the palette quits the session.
- [x] 1.4 Test: Esc closes the palette and leaves the composer text unchanged.
- [ ] 1.5 Test: `/quit` typed in the composer still quits, with the palette code present. Not yet written; the slash path is unchanged in code.
- [x] 1.6 Implement: the `Key::Ctrl('p')` handler in `src/tui.rs` that opens the command picker with no prefix.

## 2. Docs and validation

- [x] 2.1 List Ctrl+P in `src/help.rs` (the keys help) and in `README.md`.
- [x] 2.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 834 unit tests pass. The two `tests/tty.rs` failures predate this change. The keys-card test now checks Ctrl+P, because the new row pushes Shift+Tab to page 2.
