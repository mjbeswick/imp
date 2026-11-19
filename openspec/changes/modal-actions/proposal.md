# Proposal

## Why

The picker already has a title, a filter, and a list, but rows cannot carry
their own actions, and the slash autocomplete still draws inline rows instead
of using it. The session picker's `n`, `r`, and `d` shortcuts work only because
of a special case.

## What Changes

- Give every picker row optional shortcut keys, rendered as a footer hint.
- Apply a shortcut to the highlighted row without closing the picker, so
  rename/delete-style actions can open their own prompt and return.
- Use the picker for slash-command autocomplete: title, filtered list, footer
  with `Enter choose · Esc close`.
- Keep the session picker's `n`, `r`, and `d` shortcuts as row actions rather
  than filter characters when the filter is empty.

## Capabilities

### Modified Capabilities

- `picker`: rows carry actions, and the overlay is reused for autocomplete.
- `tui`: the slash autocomplete requirement describes the overlay.

## Impact

- `src/tui.rs`: picker row actions, footer, autocomplete wiring.
- `openspec/specs/picker/spec.md` and `openspec/specs/tui/spec.md`.
