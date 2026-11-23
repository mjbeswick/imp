# Design

## Approaches

- **Select inside imp (chosen).** Works with mouse capture on, which the
  wheel and the click targets need.
- **Leave selection to the terminal.** Needs capture off, losing the wheel
  and clicks, which is what Ctrl+G already offers.

## Details

`?2004h` turns on bracketed paste and `?1002h` drag reports; both are turned
off wherever the mouse and screen are restored. The decoder turns
`ESC[200~ … ESC[201~` into `Key::Paste`, waiting for the end marker when a
paste spans reads, and SGR drags and releases into `Key::Drag` and
`Key::Release`.

A selection runs in text order from the press to the pointer, over the rows
last drawn. Selected rows draw as plain text with the span reversed.
`ponytail:` one column per character; wide characters shift the span.
