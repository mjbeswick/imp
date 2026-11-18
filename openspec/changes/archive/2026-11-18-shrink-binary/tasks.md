# Tasks

## 1. Measure and trim

- [x] 1.1 Record the baseline: binary bytes, `__text` size, and the release profile in use. Verify: numbers written into the change notes and `README.md`.
- [x] 1.2 Cost display without float formatting: compute micro-dollars as integers and render with integer division. Verify: pricing tests still pass, display keeps four decimals, size measured before and after.
- [x] 1.3 A/B the profile: `opt-level` `z` versus `s`, `strip = "symbols"`, `debug-assertions = false`, `incremental = false`. Verify: the smaller setting is kept and the difference recorded.
- [x] 1.4 Linker flags `-Wl,-dead_strip` and `-Wl,-x`. Verify: binary builds, runs `--version`, and the size delta is recorded.
- [x] 1.5 Trim `serde_json` default features if it measures smaller. Verify: all JSON tests pass and the delta is recorded.
- [x] 1.6 Lower the `scripts/size.sh` budget to the achieved size plus 10%. Verify: the script passes at the new limit and fails when the limit is set below the binary.
