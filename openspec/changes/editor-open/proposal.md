# Proposal: editor commands

## Why

When the agent or the user points at a file, the user has to leave imp to open it.
Launching an editor from the chat saves that step. Terminal editors and GUI editors need
different handling, so the app needs two commands.

## What Changes

- `/edit [target]` runs a terminal editor in the foreground. The chat hands over the
  terminal, and it is restored when the editor exits.
- `/open [target]` runs a GUI editor, or the OS opener, detached. The chat keeps running.
- A target is the current directory, the file or directory selected in the picker, or a
  new file path the user types.
- The editor comes from the config keys `editor` (terminal) and `visual` (GUI). When unset,
  `$EDITOR` and `$VISUAL` are used. `/open` falls back to `open` on macOS and `xdg-open`
  on Linux.
- The editor is launched from an argument list, never through a shell.
- A file outside the session directory asks first, as `read` and `edit` do.

## Out of Scope

- Per-editor argument presets, such as `code --wait`. The user sets flags in the config.
- Opening a real editor in the test suite.
- Changes to the file tools or the picker's selection behaviour.
