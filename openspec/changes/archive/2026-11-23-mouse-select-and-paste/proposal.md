# Proposal

## Why

Pasting several lines types them as keys: the first line break is Enter, so
the first line is sent alone. With mouse capture on, a drag selects nothing,
so text on screen can be copied only through copy labels, `/copy`, or Ctrl+G.

## What Changes

- Bracketed paste is on: a paste arrives whole and goes into the draft,
  line breaks included, or into the open text prompt or answer row.
- A paste never answers a confirmation.
- Drag reporting is on: dragging highlights text on screen, and releasing
  copies it, as a copy label does, to imp's buffer and the terminal clipboard.
- A click without a drag acts as before.

## Keyboard and resources

No keys change; Ctrl+V and Ctrl+G stay. A drag repaints only the rows it
covers. On the Hackberry the trackpad drags like a mouse.
