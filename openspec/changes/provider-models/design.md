# Design

## Context

- `NamedProvider` (src/config.rs, around line 117) holds one `model`. `providers_block`
  (around line 639) writes one `provider.<name>.<field>` line per field inside the
  managed block, and `save_named` replaces that block.
- `model_items` (src/tui.rs, around line 3844) builds the `/model` list from the
  session's current model, the default for its provider, the models the session has
  seen, and recent models.
- `set_provider` (src/tui.rs, around line 4861) applies a provider to the session by
  name, with its dialect, base URL, model and key variable.
- The provider form (`Pending::ProviderForm`) and its finish step
  (`finish_provider_form`) already save the four current fields.

## Decision

- `NamedProvider` gains `models: Vec<String>`. Parsing reads `provider.<name>.models`
  as comma-separated; `providers_block` writes it back, omitting the line when empty.
- The provider form gets a fifth field, `models`. Its value is split on commas and
  trimmed. The default model is added to the list at save time if missing.
- A built-in provider's configuration still saves as a named copy, which now carries
  its models.
- `model_items` adds one row per (provider, model) pair across the built-in and named
  providers. Each row's value holds the provider name and model, and the row shows the
  provider. The session's current model stays first and marked.
- Choosing a row calls `set_provider` with that provider, then sets the model. Recent
  models stay in the recents file, so the row for a recent model still carries its
  provider when that provider is known.

## Risks

- A model name listed under two providers appears twice. That is intentional, since
  the rows differ by provider.
- A provider's models are only what the user typed. A model the list does not hold
  needs the form.
