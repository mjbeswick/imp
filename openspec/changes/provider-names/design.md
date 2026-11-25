# Design: provider names

## Context

- The form is opened by `configure_provider` (`src/tui.rs:5000`) for a saved profile or a built-in,
  and by `provider add` (`src/tui.rs:4870`). The form's name is held in `Pending::ProviderForm { name, .. }`
  and is not one of its `fields`, so it cannot be typed into. The form's fields are base, model,
  dialect, key_env, models, and api_key.
- A built-in copy is named `format!("{value}-custom")` (`src/tui.rs:5025`).
- `provider add` refuses a built-in name (`Provider::parse`, `src/providers/mod.rs:63`) and a name already
  saved (`src/tui.rs:4882`). Neither check runs in the form.
- Name validity is `valid_profile_name` (`src/config.rs:773`): non-empty, at most 40 characters, letters,
  digits, `-`, and `_`.
- Saving the form goes through `finish_provider_form`. Its save path is not yet read in full, and the
  rename case is part of this change.

## Decisions

- **Default name.** `default_provider_name(dialect, taken)` returns the dialect, then `<dialect>-2`,
  `<dialect>-3`, and so on, until the name is free. `taken` is every built-in name and every saved name.
  This replaces the fixed `-custom` suffix.
- **Name field.** The name becomes the first field in the form, labelled `name`, pre-filled with the
  default. Its current value is saved on Enter.
- **One rule.** `check_provider_name(candidate, editing: Option<&str>, named, builtins)` in `src/config.rs`
  is the only place that decides a name is taken. It returns an error string or nothing. A profile
  editing itself passes its own name as `editing`, so an unchanged name is accepted.
- **Where it runs.** `provider add`, configure-to-copy, and the form save. The form shows the error and
  keeps the form open, the same way it shows other field errors.

## Risks

- A rename does not move the profile's entries elsewhere (the active provider, favourites, and recents
  refer to `provider::model` and are not updated). Out of scope, as the proposal says. A rename of the
  active profile must keep `cfg.active_name` consistent, which the implementation must check.
- `README.md` line 166 describes the old copy name. It must be updated in the same change.

## Assumptions

- The rename is a form field, so a rename happens only when the profile is saved from the form.
- The default name for a built-in copy follows the same rule, but every built-in name is taken, so a copy
  of `deepseek` defaults to `deepseek-2`, and a copy of `custom` defaults to `custom-2`.
