# Proposal

## Why

The TUI asks the terminal for disambiguated keys, so iTerm2 reports Ctrl+C as
`CSI 99;5u` rather than the raw 0x03 byte. The decoder understands only Enter,
Tab, Esc, and Backspace in that form, so every Ctrl shortcut is dropped: Ctrl+C
does not exit, and Ctrl+D, Ctrl+L, Ctrl+O, Ctrl+T and Ctrl+H do nothing either.

Tests passed because they feed `Key::Ctrl('c')` straight into the key handler, or
send a literal 0x03, which never exercises the path a real terminal takes.

## What Changes

- Decode `CSI <codepoint> ; <modifier> u` for letters with the Ctrl bit set into
  `Key::Ctrl(<letter>)`.
- Keep the existing Enter, Shift+Enter, Tab, Esc, and Backspace mappings.
- Add decoder tests for `CSI 99;5u` and a couple of the other Ctrl keys.

## Capabilities

### Modified Capabilities

- `tui`: modified keys are decoded, so the documented Ctrl shortcuts work.

## Impact

- `src/tui.rs`: `disambiguated`.
- `openspec/specs/tui/spec.md`: keyboard requirement scenario.
