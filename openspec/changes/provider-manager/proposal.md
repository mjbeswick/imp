# Proposal: provider manager

## Why

Providers are managed in several places: the `/provider` picker, the provider form, the
`provider.<name>.enabled` config switch, and the `/model` list. Enabling, disabling,
deleting, and editing a provider each take a different route, and there is no single
place that shows every provider and its state.

## What Changes

- The providers list modal, opened by `/provider`, lists every built-in and named provider
  with its state: active, no key, or off.
  - Enter opens the provider's config.
  - `u` makes a provider active.
  - `space` enables or disables it.
  - `d` deletes a named provider. Built-in providers can only be disabled, not deleted.
  - `n` adds a new named provider.
- The provider config modal edits one provider's base URL, model, key variable, and dialect.
  Save writes the config and closes the modal. Cancel discards the edits and closes it.
  A key typed in the modal is stored in the keys file, never in the config.
- The `/model` list stays for choosing a model. Its rows already name their provider, and
  it gains no provider actions.

## Out of Scope

- Changing how providers are called, or the dialects they speak.
- Changing the built-in provider profiles.
- A separate file format for providers. The config's existing lines are used.
