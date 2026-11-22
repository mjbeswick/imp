# Design

## Approaches

- **Cross-build on the Mac (chosen).** Fast, and only the Mac needs extra
  toolchains.
- **Keep building on the device.** Simple, but slow and RAM-bound.

## Risk

imp links libcurl. An aarch64 Linux build on macOS needs a linker and a
sysroot holding the device's `libcurl.so` and glibc. The plan copies those
from the device once into `target/sysroot/` and links against them.

## Decisions

- **Toolchain: `cargo-zigbuild`.** One tool, and it pins the glibc version
  to the device's, so the binary loads there. Rejected: a rustup target plus
  a Homebrew linker, which needs the glibc pin set by hand.
- **Measure by hand.** There is no CI; `measure.sh` runs when figures change.
- **Stable Rust only.** Releases do not use nightly or `build-std`.
