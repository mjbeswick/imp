# Design

## Approaches

1. **Request disambiguated keys and parse them.** On entering raw mode send
   `CSI > 1 u` to push kitty keyboard flag 1, and `CSI < u` on restore. Flag 1
   only disambiguates keys that are otherwise ambiguous, so Enter, Esc, Tab, and
   backspace keep their legacy bytes when unmodified and Shift+Enter arrives as
   `CSI 13 ; 2 u`. Chosen: it is the only option that makes Shift+Enter work
   without per-machine setup, and it costs one decoder branch.
2. **Key mapping in the terminal profile.** No code, but only fixes machines
   someone configures, and leaves the frame corruption in place.
3. **Trailing backslash continues the line.** Works everywhere, but it is not
   what the key means and it collides with real backslashes.

Fallbacks stay: a bare line feed (Ctrl+J) and `ESC` `CR` (Alt+Enter) already
decode as newlines, which covers terminals that ignore the request.

## Decisions

- Row budget is `min(8, rows / 2)`. The status line, the slash menu, and at
  least one transcript row always survive.
- The caret row scrolls into view: when the input exceeds the budget, the
  composer shows its last rows.
- Rows are truncated from the left with a leading ellipsis so the caret stays
  visible on a long line. No wrapping, matching the one-column rule.
- Up and Down compute the caret's row and column. They move to the same column
  on the neighbouring row, and only fall back to history when no row is left.
- Decoding gets a generic CSI scanner: drain through the final byte in
  `0x40..=0x7e`. Known finals keep their meaning, `u` reads
  `codepoint;modifier`, and anything unrecognised becomes `Unknown` instead of
  leaking characters into the input.
- The protocol release is idempotent and runs in the same restore path as raw
  mode, so panic and signal exits also undo it.
- Erase below the status row happens before the status text, not after. With
  wrapping disabled the cursor rests on the last cell of a full row, so an erase
  there deletes that cell, which is the ellipsis on a truncated status.
