# Design

## Approaches

- **A hard limit in a file (chosen).** Growth needs a visible edit.
- **Keep reporting only.** The current choice; it let size grow 70%.
- **Fail on any growth.** Too strict; every feature costs bytes.

## Details

The limit lives in `scripts/size-limit.txt`. `-Z build-std` needs nightly, so
it is only practical once builds happen on the Mac (`pi-release`).
`panic=immediate-abort` is rejected: it skips the panic hook and breaks "The
terminal is always restored".

## Open questions

- The limit: 1 MB, or the current size plus a margin?
- Adopt `build-std` and nightly for releases, or stay on stable?
