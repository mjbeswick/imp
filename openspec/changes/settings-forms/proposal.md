# Proposal

## Why

The `/settings` modal shows ten settings. Each row steps through a short list of
values, which suits the enumerable ones. The config accepts about two dozen
keys, and the rest can only be changed by editing the config file. Free-text and
number settings, such as `model`, `max_tokens` and `skills_dir`, need a value
typed in. The modal already opens forms for providers and for keys, so the same
pattern can cover them here.

## What Changes

- The modal gains four enumerable rows: `allow_sudo` (on or off), `color` (auto,
  always or never), `links` (auto, always or never), and `title_summary` (auto,
  on or off).
- A text or number row opens a one-field form. Enter saves a valid value and
  applies it at once. A value that does not parse keeps the form open with the
  reason. Esc cancels and saves nothing.
- `api_key` appears as a row showing only `set` or `not set`. Editing it opens a
  masked field. The value is never shown in the modal, the transcript, or the
  status line, and it is stored where provider keys are stored, not in the
  config file.
- Grouped and list settings stay in the config file: `auth`, `render`,
  `pass_env` and `allow`. The modal does not show them.

## Out of Scope

- Provider settings, which stay in `/provider`.
- Multi-field forms for grouped settings.

## Keyboard and resources

No new keys beyond those the modal already uses: Enter, Esc, Left and Right.
