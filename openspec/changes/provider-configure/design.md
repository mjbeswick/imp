# Design

## Context

- `provider_items` (around line 3990 in `src/tui.rs`) builds the rows. Built-in
  rows have no actions; named rows have none either. Sessions already use
  `PickItem.actions` for single-key actions, drawn in the footer (around line
  2362).
- `/provider edit <name>` (around line 4680) opens `Pending::ProviderForm`
  with the profile's base, model, dialect and key variable. It only works for
  named profiles.
- Enter on a provider row runs `set_provider` (around line 5616).
- The form (`finish_provider_form`) validates and saves through `save_named`,
  which writes the managed block of the config.

## Decision

- Give every provider row an `e` action. The `e` handler opens
  `Pending::ProviderForm`, seeded from the named profile when one exists, else
  from the built-in profile's base URL, model and dialect, with `key_env` set to
  the built-in's key variable.
- On save, a name that matches a built-in is still saved as a named profile.
  Built-in profiles are never edited in place, so the built-in default stays
  available.
- Enter behaviour and the picker footer for other kinds are unchanged.

## Risks

- A named copy named `deepseek` shadows the built-in in the picker. The form
  refuses a name that is a built-in, and the copy is named `deepseek-custom`
  by default, which the user can change.
