# Design

## Approaches

- **Cross-build on the Mac (chosen).** Fast, and only the Mac needs extra
  toolchains.
- **Keep building on the device.** Simple, but slow and RAM-bound.

## Risk

imp links libcurl. An aarch64 Linux build on macOS needs a linker and a
sysroot holding the device's `libcurl.so` and glibc. The plan copies those
from the device once into `target/sysroot/` and links against them.

## Open questions

- Toolchain: `cargo-zigbuild` (one tool, pins glibc) or rustup target plus a
  linker from Homebrew. Leaning zigbuild.
- Should measure.sh run in CI or by hand only?
