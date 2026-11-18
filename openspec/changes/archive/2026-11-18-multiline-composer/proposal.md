# Proposal

## Why

Shift+Enter is the usual way to start a second line, and imp drops it: most
terminals send Shift+Enter as the same carriage return as Enter, so a
multi-line prompt is sent early. The parser can already produce a newline key,
but the composer draws a single row, so a newline corrupts the frame.

## What Changes

- Ask the terminal for disambiguated key encoding on entry and release it on
  exit, so Shift+Enter arrives as its own key.
- Decode `CSI <codepoint>;<modifiers> u` and map Shift+Enter to a newline.
- Keep carriage-return-free line feed and Alt+Enter as newline encodings, for
  terminals that never answer the request.
- Draw the composer as one or more rows, with the caret on its own row, and
  shrink the transcript by the rows the composer uses.
- Move Up and Down between composer rows before falling back to history.
- Update the key list in help and the `tui` spec.

## Capabilities

### Modified Capabilities

- `tui`: Shift+Enter starts a new line, and the composer takes more than one
  row.

## Impact

- `src/tui.rs`: key decoding, frame layout, composer rendering, raw-mode setup
  and restore.
- `src/help.rs`: the key list.
- `openspec/specs/tui/spec.md`: keyboard, chat column, and history
  requirements, plus one new requirement.
- No dependencies, configuration, data, or wire changes. Terminals that do not
  answer the request keep Alt+Enter and Ctrl+J.
