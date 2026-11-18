# Proposal

## Why

The release binary is 589 KB. Most of that is ours plus `std` and `core::fmt`,
and a few build choices cost more than they need to. This is a measurement
exercise: change one thing at a time, keep only what the numbers support.

## What Changes

- Replace float formatting for cost with integer micro-dollars.
- A/B `opt-level` `z` versus `s`, and confirm `panic = "abort"`, `lto`, `strip`.
- Linker flags: `-dead_strip`, `-x`.
- Trim `serde_json` features if it measures as a win.
- Record the baseline and every step's result in `README.md`.
- Drop the size budget in `scripts/size.sh` to the achieved size plus headroom.

## Capabilities

None. Build configuration only, with `skip_specs: true`.

## Impact

- `Cargo.toml`, `src/pricing.rs`, `src/tui.rs`, `scripts/size.sh`, `README.md`.
- No behaviour change; the cost display keeps four decimals.
