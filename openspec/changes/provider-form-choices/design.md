# Design

## Context

- `Pending::ProviderForm { name, fields, at }` holds the four fields as
  `(label, text)` pairs. `finish_provider_form` refuses a dialect that
  `Provider::parse` rejects.
- Settings rows already step through a list with `setting_choices` and
  `step_setting`.
- `/model` uses `model_items` and recent models stored in a file read by
  `load_models`.

## Decision

- The dialect field is stepped, not typed. Left, Right and Space move through
  `deepseek`, `openai`, `anthropic`, `gemini`, `ollama`, `custom`. Typed
  characters on that field are ignored, so `finish_provider_form` still
  validates, but a bad value cannot be reached.
- Tab on the model field opens a `PickKind::Model` picker. Its rows are the
  default model for the dialect (from its built-in profile) and the recent
  models, then a row `type another`. Choosing a row writes it into the field and
  returns to the form. `type another` returns to the form with the field focused.
- Free text on the model field keeps working.

## Risks

- The model picker lists only models the user has already used or the default.
  A new model still needs typing.
