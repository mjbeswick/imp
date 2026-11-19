# Proposal

## Why

`imp find the biggest file` sends `file` to the model. Every bare argument
overwrites the previous one in the parser, so only the final word survives. A
session log shows a user message of exactly `file`, which the model then asked
to clarify.

## What Changes

- Collect every positional argument and join them with spaces into one prompt.
- Keep flags and their values out of the joined text.
- Add a test that reproduces unquoted multi-word prompts.

## Capabilities

None. This restores the documented `imp [OPTIONS] [PROMPT]` behaviour, so the
change uses `skip_specs: true`.

## Impact

- `src/main.rs`: argument parsing and one test.
- `README.md`: an unquoted example.
