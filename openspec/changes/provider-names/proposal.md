# Proposal: provider names

## Why

A provider profile has a name, but the name is handled inconsistently. Configuring a built-in
provider always creates a copy named `<built-in>-custom` (`src/tui.rs:5025`), so a built-in `custom`
profile becomes `custom-custom`. The name cannot be edited in the provider form, and the duplicate
check runs only on `provider add` (`src/tui.rs:4882`). Two profiles can end up looking the same, and
the user cannot choose a meaningful name.

## What Changes

- Every new provider profile gets a default name taken from its dialect: `custom`, `openai`, and so on.
  If that name is taken, a numeric suffix is added (`custom-2`, `custom-3`).
- The name is an editable field in the provider form, shown as its last field so the existing field
  positions stay the same, and pre-filled with the default.
- A name is refused, with an error row, when it is a built-in provider name or belongs to another saved
  profile. The check runs on add, on configure-to-copy, and on rename in the form. A profile may keep
  its own name when it is saved again.
- Names keep the existing rules: letters, digits, `-`, and `_`, at most 40 characters.

## Out of Scope

- Changing when the built-in `custom` and `ollama` providers are listed. That is a separate change.
- Changing the active provider's key, favourites, or recent models when a profile is renamed.
- The `/provider add <name>` command syntax.

## Impact

- `src/tui.rs`: default name for new and copied profiles, the name field in the form, and the duplicate
  check in `finish_provider_form`.
- `src/config.rs`: a shared helper that checks a candidate name against built-in and saved names.
- `src/tui/tests.rs`: tests for the default name, suffix, refusal, and rename.
- `README.md`: the sentence that says saving a built-in provider adds a copy named `deepseek-custom`.
