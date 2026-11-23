# Proposal

## Why

Thinking and effort are different controls: a switch for a separate
reasoning phase, and a dial for how much work the model does. imp has only
the switch. OpenAI-compatible requests always send `reasoning_effort: "low"`,
and Anthropic requests send neither control, so `/think` does nothing there.

## What Changes

- `/effort low|medium|high` sets the dial for the session; bare `/effort`
  shows it; `/effort default` returns to the provider's own default.
- An `effort` config key and an `effort:` agent field set it too.
- OpenAI-compatible requests send the chosen `reasoning_effort` instead of a
  fixed `low`; unset keeps today's `low`.
- Anthropic requests send `output_config.effort` when set.
- A provider without a dial says so once per session when effort is set.
- `/think` and Ctrl+T stay the switch.

## Keyboard and resources

No new key: `/effort` and its palette row. Higher effort costs tokens and
time on the provider, not on the device.
