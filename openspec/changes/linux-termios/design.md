# Design

## Approaches

1. **A per-platform struct and per-platform flag type.** Matches libc exactly and
   costs two `cfg` blocks. Chosen.
2. **Use the `libc` crate.** Correct by construction, but adds the first
   dependency beyond `serde_json` and grows the binary.
3. **Shell out to `stty raw -echo`.** Depends on `stty` at runtime, cannot read
   the original settings back reliably, and hides errors.

## Decisions

- Linux layout: `iflag`, `oflag`, `cflag`, `lflag` as `u32`, then `c_line: u8`,
  `cc: [u8; 32]`, `ispeed: u32`, `ospeed: u32`. That is 60 bytes on glibc, which
  is what the kernel copies.
- A `Flag = u32` alias on Linux and `Flag = u64` on macOS, so `raw.iflag &=
  !flags::IFLAGS` compiles without casts at every site.
- `VMIN` and `VTIME` indices already differ per platform and stay as they are.
- The integration test's copy of the struct gets the same treatment, otherwise
  the test asserts against a layout the binary does not use.
- A Linux test asserts `size_of::<Termios>() == 60`. Cheap, and it fails loudly
  if someone reorders fields.
- Verified by building and typing into the binary on the Pi, not just by tests on
  the Mac, because the Mac cannot exercise the Linux layout.
