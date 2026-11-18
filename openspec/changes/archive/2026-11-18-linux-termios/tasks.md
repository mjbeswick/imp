# Tasks

- [x] 1.1 Declare `Termios` per platform and type the flag constants to match. Verify: `cargo check` passes on macOS, and `cargo build` passes on the Pi with `size_of::<Termios>() == 60`.
- [x] 1.2 Mirror the layout in the pty test. Verify: `cargo test` still passes locally.
- [x] 1.3 Prove typing works on the Pi. Verify: over an ssh pty, typing `hello` appears in the composer before Ctrl+C exits.
- [x] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
