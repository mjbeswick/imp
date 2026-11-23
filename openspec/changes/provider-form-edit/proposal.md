# Proposal

## Why

The provider form used by `/provider add` and `/provider edit` moves one way
only. Enter advances to the next field, Backspace edits only the field under
the cursor, and Esc cancels the whole form. A mistake in `base` or `model`
discovered at `key_env` means starting again.

## What Changes

- While the form is open, ↑ and Shift+Tab move to the previous field, and ↓
  moves to the next. Each field keeps its text.
- Enter still advances and saves from the last field.
- Esc still cancels the whole form and saves nothing.
- The same form is used when the first-run setup asks for a provider, so the
  keys work there too.

## Out of Scope

- Making `dialect` a choice list. It stays free text.
- A review screen before saving.

## Keyboard and resources

Two keys are added to the form: ↑/↓ and Shift+Tab. No new screen and no
request.
