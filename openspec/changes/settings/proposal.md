# Proposal

## Why

Settings live in a text file most users never open. Several have commands of
their own, others have none, and nothing shows them together. The title bar
is flat by design; a gradient was asked for as an option.

## What Changes

- `/settings` opens a form: ten settings with their current values. Right,
  Enter or Space steps a value, Left steps back, Esc closes. A change applies
  at once and is saved to the config file, comments kept.
- `title_gradient`, off by default: on a truecolor terminal the title bar
  shades from the select colour toward the accent. Elsewhere it stays flat.
- `/settings` makes twenty-eight commands; it is in the palette.

## Keyboard and resources

Arrows, Enter, Space and Esc, all on the Hackberry. The gradient writes one
colour per cell, about 1.5 KB per title repaint, once a minute; it is off by
default for that reason.
