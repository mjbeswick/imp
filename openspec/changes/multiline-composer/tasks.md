# Tasks

- [x] 1.1 Decode `CSI <codepoint>;<modifiers> u` and map Shift+Enter to a newline. Verify: unit tests cover `ESC [ 13 ; 2 u` as newline, `ESC [ 13 u` as send, `ESC [ 27 u` as Esc, and an unknown final draining cleanly.
- [x] 1.2 Push the keyboard protocol on entry and release it on restore. Verify: the pty test sees `ESC [ > 1 u` before the first frame and `ESC [ < u` after quit.
- [x] 1.3 Draw the composer across rows with the caret on its row and a row budget of `min(8, rows / 2)`. Verify: a frame test shows two rows, the status last, and no bare line feed.
- [x] 1.4 Move Up and Down between composer rows before history. Verify: a key test moves within rows and only then recalls history.
- [ ] 1.5 Update the key list in `help.rs` and the `tui` spec. Verify: `imp --help` names Shift+Enter and `openspec validate --all --strict` passes.
- [ ] 1.6 Run the checks. Verify: `cargo test` and `./scripts/size.sh` pass.
