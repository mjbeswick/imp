# Design

## Approaches

1. **Map the Ctrl bit to `Key::Ctrl(letter)`.** One branch in `disambiguated`.
   Chosen: the handler already understands `Key::Ctrl`, so no other code moves.
2. **Stop requesting the keyboard protocol.** Then Ctrl+C arrives as 0x03 again,
   but Shift+Enter stops working, which the protocol was added for.
3. **Normalise in the key reader thread.** Moves terminal knowledge into the I/O
   thread and leaves the decoder untested.

## Decisions

- Modifiers are one plus a bit set: 1 shift, 2 alt, 4 ctrl. Only ctrl is mapped,
  since the handler has no alt shortcuts and shift is already handled for Enter.
- Lowercase the codepoint before mapping, so Ctrl+Shift+C also lands as Ctrl('c').
- Unknown codepoints stay `Unknown` rather than becoming typed characters, which
  would leak escape parameters into the composer.
- The regression test sends the exact bytes iTerm2 produces, not `Key::Ctrl('c')`.
