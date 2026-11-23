# Proposal

## Why

Each provider is configured with one model, and `/model` only lists models for the
provider the session is already using. To use a model from another configured
provider, the user has to switch provider first, then switch model. A provider can
offer several models, but there is nowhere to record them.

## What Changes

- A provider's form gains a `models` field: a comma-separated list of model names.
  The default model is always offered, whether or not it is in the list.
- `provider.<name>.models` is saved in the managed providers block of the config.
- `/model` lists every model of every configured provider, built-in and named, plus
  recently used models. Each row shows its provider, for example
  `deepseek-pro · deepseek-custom`.
- Choosing a row switches the session to that provider and model, through the same
  path as `/provider use`. The status line shows the provider.
- Models are typed by hand. Nothing is fetched from a provider.

## Out of Scope

- Fetching model lists from providers.
- Using two providers in one turn.

## Keyboard and resources

No new keys. The provider form gains one field.
