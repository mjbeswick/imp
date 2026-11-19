# Proposal

## Why

A turn stops after exactly 24 completion/tool cycles. The number is hardcoded in
the loop and written into the spec, so a slow task that legitimately needs more
cannot finish, and an operator cannot tighten the budget for a cheap model or a
noisy one.

## What Changes

- Add `max_steps` with the current 24 as the default.
- Expose it as `IMP_MAX_STEPS` and `--max-steps`; reject values below 1 and cap
  absurd values at 1,000.
- When the limit is reached, execute the pending tool batch, add a visible note
  that the turn paused, keep the history and partial answer, and leave the
  input ready. No dangling tool call without a result.
- Transport retries and context-overflow retries stay inside their step and do
  not consume the budget.
- Update the `agent-loop` spec, `--help`, and the README.

## Capabilities

### Modified Capabilities

- `agent-loop`: the iteration limit is configurable, and reaching it pauses the
  turn instead of failing it.

## Impact

- `src/config.rs`: the setting, environment, and validation.
- `src/agent.rs`: the loop reads the configured limit and pauses cleanly.
- `src/main.rs` and `src/help.rs`: the flag and docs.
- `README.md` and `openspec/specs/agent-loop/spec.md`.
