# Design

## Approaches

1. **Row actions on the existing picker.** Chosen: one overlay, one key path,
   and the session shortcuts stop being a special case.
2. **A second action menu after selection.** More keystrokes and two overlays.
3. **Global shortcuts per picker kind.** Duplicates configuration and hides the
   available actions from the user.

## Decisions

- `PickerRow { label, value, actions: Vec<(char, &'static str)> }`. The footer
  renders `key action` pairs next to `Enter choose · Esc close`.
- Actions run against the highlighted row and leave the picker open its state;
  an action that needs input opens the existing prompt and returns.
- A printable character is an action only when the filter is empty and a row
  declares it. Otherwise it appends to the filter. Backspace always edits the
  filter.
- Slash autocomplete becomes a picker with `Kind::Commands`: the current input
  after `/` is the initial filter; Enter runs, Tab completes, Esc closes.
- The session picker moves `n`, `r`, `d` from its special-case handler into row
  actions. Agent and verbosity pickers declare none.
- The overlay remains the only bordered element and reuses the same compositor.

## Tests

- Shortcut runs on the highlighted row; the same key filters when text exists.
- Footer lists the declared actions.
- Slash overlay filters, Tab completes, Enter runs, Esc leaves the text.
