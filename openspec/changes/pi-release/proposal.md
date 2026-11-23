# Proposal

## Why

imp targets the Hackberry Pi Zero 2 W, yet its figures come from an M1 and
`deploy.sh` compiles on the device, which is slow with 463 MB of RAM. The
README suggests tmux or dtach; the Hackberry needs neither (decided
2026-11-23), and the rock5b already has tmux.

## What Changes

- `scripts/measure.sh` reports binary size, cold start and idle RSS, run
  locally or on a host. The README lists M1 and Pi figures side by side.
- `deploy.sh` builds an aarch64 binary on the Mac and copies it, keeping
  `imp.prev`. The on-device build stays as `--native`.
- Device task: walk the keys card on the Hackberry's own
  keyboard, listing keys it cannot type.

## Keyboard and resources

Keys unchanged; the keyboard walk may feed a later change. Deploys stop
using the Pi's CPU and RAM for compiling.
