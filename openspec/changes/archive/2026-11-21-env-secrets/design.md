# Design

## Decisions

- **By name, read once.** Secret variables are found by name when first
  needed and cached; the environment does not change while imp runs.
- **Scrub at the spawn.** `env_remove` on the agent's `sh -c` and on
  background jobs. A superuser command resets the environment itself, so the
  attached password path needs nothing.
- **Redact values of eight characters or more.** Shorter values would match
  ordinary words. Redaction covers every tool result and typed command output.
- **`pass_env` is process-wide,** set once at startup, so `Tools` and `Jobs`
  need no new field.
- **Typed commands are the user's.** They keep the environment; only their
  output is redacted.

## Keyboard

No change.

## Resources

One string search per secret per tool result. A few secrets, so negligible on
the Pi.
