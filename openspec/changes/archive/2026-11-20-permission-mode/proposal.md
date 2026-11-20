# Proposal

## Why

`/yolo` only toggles between `yolo` and the launch mode. There is no way to
tighten to `strict` mid-session, and nothing shows the current mode unless it
is `yolo`. Every other setting with a few values uses the value picker.

## What Changes

- **BREAKING** `/yolo` is replaced by `/mode`.
- Bare `/mode` opens the value picker with `normal`, `strict`, and `yolo`, each
  with a one-line description, and the current mode marked `current`.
- `/mode <name>` sets the mode directly; Tab completes it like other value
  commands. An unknown name is an error that lists the three.
- The change lasts for the session and is not saved to the config.
- Line mode gets the same `/mode`, printing the aligned list with `current`.
- The yolo colour, the deny list, and the sudo gate are unchanged.

## Capabilities

### Modified Capabilities

- `tui`: the command list, mode switching, and the yolo colour scenarios.
- `picker`: `/mode` joins the value commands.

## Impact

- `src/tui.rs`, `src/main.rs`, `src/agent.rs`, `src/help.rs`, `README.md`.
