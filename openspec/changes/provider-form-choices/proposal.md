# Proposal

## Why

The provider form takes the dialect and the model as free text. The dialect has
six valid values, so a typo is only caught when the form is saved. The model
name changes often, but the user has already picked models with `/model`, and
those choices are not offered here.

## What Changes

- The dialect field is a choice of `deepseek`, `openai`, `anthropic`, `gemini`,
  `ollama`, or `custom`. Left, Right and Space step through the values; typed
  text is ignored. A dialect that is not in the list cannot be entered.
- The model field keeps free text. Tab on the model field opens a picker of the
  dialect's default model and the recently used models, with a last row to type
  another name. Choosing a row fills the field.
- The base URL and key variable stay free text.

## Out of Scope

- Fetching the model list from the provider.
- Changing `/model`.

## Keyboard and resources

Tab on the model field opens its picker. Left, Right and Space step the dialect.
No new screen beyond the existing picker.
