# Proposal: provider opt-in

## Why

Ollama and Custom are built-in providers that need no API key. They are always listed in `/provider`
and in `/model`, even when the user has never set them up. Custom then appears next to the user's own
profiles, which looks like a duplicate. The keyed built-ins (DeepSeek, OpenAI, Anthropic, Gemini) are
listed only once their key is set, so the keyless ones are the odd case.

## What Changes

- Ollama and Custom are hidden from `/provider` and `/model` until they are switched on.
- They are switched on with `provider.<name>.enabled = true` in the config.
- The active provider is always listed, even when it is hidden. A switched-off provider is never listed.
- The keyed built-ins keep their current rule: listed once the key is set.

## Out of Scope

- The keyed built-ins and their key handling.
- A picker control to switch the keyless built-ins on. Switching on is done in the config for now.
- Naming rules for saved profiles, which are covered by the `provider-names` change.

## Impact

- `src/config.rs`: a list of switched-on providers, parsed from `provider.<name>.enabled`, and
  `provider_listed`, which decides whether a built-in is offered.
- `src/tui.rs`: the model list and the provider list use `provider_listed`, and the provider list skips a
  hidden keyless built-in.
- `src/tui/tests.rs`: the test fixture switches Ollama and Custom on so the existing provider-row tests keep
  their setup, and a new test checks the hidden default.
- `README.md`: the sentence that lists the built-in providers.
