# Design

## Decisions

- **Whole command, no shell parser.** Any of `& ; > < $( `` ` `` newline asks.
  `|` splits the command, and each part must be safe on its own.
- **Quotes are not parsed.** `rg "a|b"` splits inside the quotes and asks. It
  over-asks but never under-asks.
- **Per-program flag rules** for the few safe programs that write or execute.
- **`awk` removed** rather than parsed: its scripts can call `system()`.

## Keyboard

No change. Some commands now ask; `s` and `a` still approve a shape.

## Resources

None.
