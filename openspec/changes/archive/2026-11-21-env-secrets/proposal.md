# Proposal

## Why

Every command the agent runs inherits imp's environment, provider keys
included. `echo $DEEPSEEK_API_KEY` is safe-listed, so it runs unprompted and
puts the key in the conversation, the provider's logs, and the session file.
`cat ~/.zshrc` does the same for keys exported there. Removing fetch prompts
next makes injected page content easier to reach, so this closes first.

## What Changes

- The agent's commands and background jobs run without secret-looking
  variables: names ending `_KEY`, `_TOKEN`, `_SECRET`, `_CREDENTIALS`, or
  containing `PASSWORD`, and `AWS_*`.
- `pass_env = NAME, …` in the config passes chosen ones through.
- Tool output has every secret value replaced with `[redacted NAME]` before it
  reaches the conversation, the log, or the screen.
- A command naming a stripped variable asks first, in `normal` and `strict`.
- Commands the user types with `!`, and `/shell`, keep the full environment;
  their output is still redacted.

## Capabilities

### Modified Capabilities

- `permissions`: secrets stay out of the agent's commands.

## Impact

- `src/secrets.rs` (new), `src/tools.rs`, `src/jobs.rs`, `src/agent.rs`,
  `src/config.rs`, `src/main.rs`, `src/tui.rs`, `README.md`.
