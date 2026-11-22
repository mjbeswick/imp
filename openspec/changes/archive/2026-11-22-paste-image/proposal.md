# Proposal

## Why

A screenshot has to be saved to a file and its path typed before the model
can see it. Terminals paste text only, so Cmd+V cannot carry an image; other
agents read the system clipboard on a key instead.

## What Changes

- Ctrl+V first asks the system clipboard for an image: `osascript` on
  macOS, `wl-paste` on Wayland, `xclip` on X11. An image is saved to the
  images folder and attached to the next message, with the usual marker.
- With no image there, Ctrl+V pastes imp's last copy, as now.
- Over SSH, or with no clipboard tool, Ctrl+V behaves as today; the first
  time, one line says why no image came.
- Backspace on an empty draft removes the last attached image.
- The same limits as path attachments: 10 MB, four per message.

## Keyboard and resources

Ctrl+V gains a meaning; Backspace on an empty draft gains one. A clipboard
check runs one short process, capped at two seconds. No new dependency.

## Capabilities

### Modified Capabilities

- `images`: images from the clipboard, and removing the last one.
