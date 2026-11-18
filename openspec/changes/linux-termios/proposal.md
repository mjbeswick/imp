# Proposal

## Why

The TUI cannot be typed into on the Hackberry Pi, which runs Debian on aarch64.
The interface draws, the status line updates, and keys are echoed by the
terminal, but nothing reaches the composer.

The hand-rolled `Termios` struct is the macOS shape: four `u64` flag fields and a
20-byte control-character array with no `c_line`. On Linux the same struct is
four `u32` flags, one `c_line` byte, 32 control bytes, and two `u32` speeds.
`tcgetattr` fills 60 bytes that get read at the wrong offsets, and `tcsetattr`
writes the same garbage back, so raw mode never takes effect. The keys stay in
the line discipline and the terminal keeps echoing them.

It looked fine in earlier checks because `script` gave the app a pipe for stdin,
where `tcgetattr` fails quietly and the app reads the pipe anyway.

## What Changes

- Declare `Termios` per platform: macOS `u64` flags and 20 control bytes, Linux
  `u32` flags, a `c_line` byte, and 32 control bytes.
- Type the flag constants per platform so the bit operations still compile.
- Do the same in the pty integration test, which carries its own copy.
- Add a Linux-only size assertion so the layout cannot drift again.

## Capabilities

### Modified Capabilities

- `tui`: raw mode is configured correctly on Linux, so keys are read there.

## Impact

- `src/tui.rs`: struct, flag types.
- `tests/tty.rs`: the mirrored struct.
- `openspec/specs/tui/spec.md`: terminal requirement gains a Linux scenario.
