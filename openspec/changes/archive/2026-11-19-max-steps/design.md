# Design

## Approaches

1. **A config value read by the loop.** Chosen: one field, one parse path, and
   the existing config/env/flag trio already used for other limits.
2. **A per-agent frontmatter key.** Useful later, but it would make the cap
   depend on which profile started the turn for no clear benefit.
3. **Unlimited by default.** Removes the runaway guard that protects cost.

## Decisions

- `Config::max_steps: usize`, default 24, parsed from `max_steps=`,
  `IMP_MAX_STEPS`, and `--max-steps`. Values below 1 are an error naming the
  minimum; values above 1,000 are clamped.
- `agent.rs` drops the `MAX_STEPS` constant and reads `self.cfg.max_steps`.
- The loop runs up to the configured number of completion requests. After the
  tool batch for the final step, it emits `Ui::Info("paused after N steps; send
  another message to continue")` and returns `Ok`, leaving matched tool results
  and the partial answer. It no longer returns an error from this path.
- Retries and context-overflow recovery stay inside the same step, so a flaky
  provider cannot consume the budget.
- A text response at the final step ends normally; the pause note only appears
  when tool calls would require another request.
- Help and README list the flag and the environment variable next to
  `--model` and `IMP_MODEL`.
- Tests: config parsing and bounds; a scripted transport that always returns a
  tool call with `max_steps=2`, asserting exactly two requests, matched results,
  the note, and `Ok`; resume after the pause.
