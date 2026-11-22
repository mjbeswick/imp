# Proposal

## Why

The Hackberry has a built-in mouse, and the user wants to click labels and
list rows instead of typing their keys. imp captures the mouse for the wheel,
so kmscon cannot select text inside imp; Shift-drag does nothing there. The
user needs to copy reply text into imp's prompt on the device and into apps
on the Mac over SSH.

## What Changes

- Clicks act on what they land on: picker rows, confirm chips, question
  options, and picker footer hints each do what their key does.
- `[a] always` takes a second click on the same chip; `yes` and `no` take one.
- Each assistant reply and each fenced code block shows a dim `⧉` at its
  right edge. A click copies that text.
- `/copy` copies the last reply; `/copy code` copies the last code block.
- A copy goes to imp's own buffer, which Ctrl+V pastes into the composer, and
  to the terminal clipboard over OSC 52.
- Ctrl+G releases the mouse until the next key, so the terminal's own
  selection works; that key is then handled as usual.
- The interface becomes keyboard first with the mouse optional; every click
  has a key.
- The command list grows to twenty-three: the existing twenty-two, `/goal`
  now named in the spec, plus `/copy`.
- The `/shell` requirement states that the shell starts in the session
  directory, as it already does.

## Keyboard and resources

Every mouse action keeps a key, so nothing needs the mouse. Ctrl+G and Ctrl+V
are single chords on the compact keyboard. A frame records its click targets
in one small table; a copy costs one base64 encode. No process spawns, no
new dependencies.

## Capabilities

### Modified Capabilities

- `tui`: clicks, copy labels, `/copy`, the copy buffer, mouse release, the
  command list, and the shell directory.
