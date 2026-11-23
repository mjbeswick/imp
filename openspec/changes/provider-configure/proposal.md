# Proposal

## Why

Selecting a provider in the `/provider` picker switches to it at once, and the
only way to change its base, model, or dialect is to type `/provider edit`
for a named profile. Built-in providers cannot be changed from the picker at
all, so a user who wants a different model for DeepSeek has to remember the
command and the syntax.

## What Changes

- Enter in the provider picker still switches to the highlighted provider.
- `e` on a provider row opens the provider form, the same form `/provider
  edit` opens, for that provider.
- For a custom provider, the form edits the saved profile.
- For a built-in provider, the form starts from its defaults. Saving it adds a
  named copy to the config, and the built-in profile stays as it was.
- The picker footer shows `e configure` on a provider row.

## Out of Scope

- Entering the API key value from this form. The form stores the name of the
  variable that holds the key, as it does now.

## Keyboard and resources

One key, `e`, on a provider row. No new screen.
