# Design

## Approaches

- **Ctrl+V reads the clipboard through the platform's tool (chosen).** What
  opencode does. macOS needs nothing installed; Linux has `wl-paste` or
  `xclip` on most desktops.
- **A separate key for image paste.** Keeps Ctrl+V's text paste instant,
  but is one more key to learn for the case people expect Ctrl+V to cover.
  Rejected.
- **Read images from the terminal's paste.** Terminals send text only.
  Not possible.
- **A clipboard crate.** Hundreds of KB for three shell commands. Rejected.

## Reading the clipboard

`images::clipboard_command()` picks the command: none under `SSH_CONNECTION`,
the `osascript` script on macOS, `wl-paste --no-newline --type image/png`
with `WAYLAND_DISPLAY`, `xclip -selection clipboard -t image/png -o` with
`DISPLAY`, and none otherwise. On macOS the script writes the clipboard's
`«class PNGf»` to a temporary file and prints it, so no hex decoding is
needed.

`images::read_clipboard(cmd)` runs it under `sh -c` with a two-second limit.
Exit 127, a missing tool, means unsupported. Output that is a PNG, JPEG,
GIF, or WebP is an image; anything else means no image.

`attach_in` splits into reading a path and `store(bytes, name, dir)`, so a
pasted image goes through the same checks and the same folder. Its name is
`pasted.png`.

## Chat

`App.paste_cmd` holds the command, so tests can set a fake one. Ctrl+V
tries the image before the selection logic runs, so an image paste leaves
a selection alone. The "no clipboard tool" note shows once per process.

## Removing

Backspace on an empty draft sends `Cmd::Detach`. The worker pops the last
attachment and reports `removed [image …]`; with none, nothing is said.

## Size

Expected: 1 to 2 KB.
