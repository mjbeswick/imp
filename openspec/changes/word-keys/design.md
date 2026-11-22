# Design

## Approaches

- **Readline and Option keys (chosen).** Known from every shell; no mode.
- **Optional vi bindings.** Need Esc for normal mode, and Esc stops the
  turn. Rejected for now.

## Keys

The decoder reads `ESC b`, `ESC f`, `ESC d`, and `ESC DEL` as Alt keys,
`CSI 1;3D` and `CSI 1;3C` as word moves, `CSI 1;4D` and `CSI 1;4C` as word
selection, and `CSI 3;3~` as delete word forward. A lone Esc stays Esc.

A word is a run of letters, digits, or `_`; a move skips the spaces and
punctuation before it, as readline does. Ctrl+A, Ctrl+U, and Ctrl+K act on
the caret's row of the draft, not the whole draft.

The selection keys reuse `composer-select`'s anchor.

## Size

Expected: about 1 KB. Measured: `__text` +1104, `__cstring` +0.
