# Proposal: provider toggles

## Why

The `/model` list shows every built-in provider, OpenAI included, even when the user has
no key for it. Choosing one of those rows switches to that provider and then demands its
key, so the app appears to expect `OPENAI_API_KEY` to be set.

## What Changes

- A provider is enabled by default only when it is configured: its key variable is set,
  or it needs no key (Ollama, Custom), or it is a named provider in the config.
- `provider.<name>.enabled = false` switches a configured provider off.
- The `/model` list shows only enabled, configured providers. An unconfigured OpenAI does
  not appear and does not ask for `OPENAI_API_KEY`.
- A provider switched off is left out of the list and cannot be made active from it.

## Out of Scope

- A picker screen for toggling providers. The config key is the switch for now.
- Changing how a key is stored or read.
- Changing the built-in provider profiles.
