# Tasks

- [x] 1.1 Add `max_steps`. Config default 24, `max_steps=`, `IMP_MAX_STEPS`, `--max-steps`; reject below 1 and clamp above 1,000. Verify: defaults, a custom value, 0 rejected, 5000 clamped.
- [ ] 1.2 Use it in the loop. Replace the constant, count completion requests, run the final tool batch, and pause with a note instead of an error. Verify: a scripted endless tool caller with `max_steps=2` makes two requests, leaves matched results, returns `Ok`, and continues on the next message.
- [ ] 1.3 Document the setting in `--help` and the README. Verify: both list the flag and environment variable.
- [ ] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
