# Design

## Context

The Hackberry runs DietPi with kmscon v10 on the console and no X or
Wayland. kmscon passes clicks to imp while mouse reports are on, and
Shift-drag does not bypass that. There is no system clipboard on the device.

## Approaches

- **Click-to-copy labels, plus mouse release (chosen).** It fits "click on
  labels", is cheap on the Pi Zero, and release covers free-form selection
  through kmscon.
- **Drag-select inside imp.** The most flexible option, but it has to map
  screen cells back to wrapped, styled text and redraw on every drag event.
  Rejected for cost and size.
- **Terminal selection only.** Almost no code, but Shift-drag does nothing in
  kmscon, so it fails on the target device.

## Click targets

`frame()` records `(row, first_col, last_col, Target)` for each clickable
span it draws, in a `Vec` on the app. A target is a key it replays
(`Key::Char('y')`, `Key::Enter` after moving the picker index, `Key::Esc`)
or a copy of entry `i` or fence `j` of entry `i`. A click looks the cell up
and replays the key through the normal handler, so mouse and keyboard share
one path.

`mouse()` decodes SGR button 0 press (`M`, not release `m`) as
`Key::Click(x, y)`. Release events and other buttons stay `Key::Unknown`.

## Always

`[a] always` sets `armed = true` on the first click and repaints the chip
reversed. A second click on it replays `a`. Any other key or click clears
`armed`.

## Copy

- The reply label copies the entry's source text. The fence label copies the
  lines between the fences, found by scanning the source, not the rendered
  rows.
- `App.clip: String` holds the copy. Ctrl+V inserts it at the caret.
- `render()` writes `ESC ] 52 ; c ; <base64> BEL` once after the frame. The
  existing `api::base64` is reused. Under tmux this needs
  `set-clipboard on`; the README says so. Terminal.app ignores OSC 52.

## Release

Ctrl+G writes `MOUSE_OFF` and sets `released`. The input loop writes
`MOUSE_ON` on the next key before handling it. The status line shows the
release notice while `released` is set.

## Size

Expected: about 3 KB added to the release binary. The measured figure goes
in the archive commit.

## Keyboard and resources

Ctrl+G and Ctrl+V are single chords, free in code and spec; raw mode clears
IEXTEN, so Ctrl+V reaches imp. The target table is rebuilt each frame and is
bounded by the visible rows. No spawns and no dependencies.
