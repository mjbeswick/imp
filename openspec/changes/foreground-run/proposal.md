# Proposal

## Why

The agent cannot start an app the user wants to use or watch, such as a TUI,
a game, or a dev server: commands run with stdin closed. imp already hands
the terminal over for password prompts, so the same path can run any
command in the foreground.

## What Changes

- The shell tool takes `foreground: true`. The command gets the terminal; imp
  takes it back when the command exits.
- The model gets only the exit code. Output is not captured.
- It always asks first in `normal` and `strict`, whatever the allows say;
  `yolo` runs it.
- Line mode refuses it.
- The system prompt allows interactive commands only through `foreground`.

## Keyboard and resources

The app's own keys apply while it runs; Ctrl+C ends a server. No capture, so
no extra process. The mouse is handed over with the terminal.

## Capabilities

### Modified Capabilities

- `tools`: foreground commands.
