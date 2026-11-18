# Tasks

- [x] 1.1 Add a test for the line-entry reader: `hello\r`, `a\r\nb\n`, and empty input. Verify: the test fails, no such reader exists.
- [x] 1.2 Read line-mode entries until carriage return or line feed. Verify: the reader test passes and `cargo test` passes.
- [x] 1.3 Add a pty test that runs the TUI, sends Ctrl+D, and asserts the terminal mode is unchanged. Verify: it fails against the current build.
- [x] 1.4 Set and restore raw mode with termios on normal exit, panic, SIGINT, and SIGTERM. Verify: the pty test passes and `cargo test` passes.
- [x] 1.5 Add a test that Esc asks a running turn to stop. Verify: the test fails, Esc only queues a command.
- [x] 1.6 Write the session cancel flag from Esc and Ctrl+C while streaming. Verify: the test passes and a manual turn stops on Esc.
- [ ] 1.7 Validate the change and size budget. Verify: `openspec validate --all --strict` and `./scripts/size.sh` pass.
