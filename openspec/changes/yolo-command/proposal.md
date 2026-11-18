# Proposal

## Why

`yolo` is reachable only at launch, through the config file or `IMP_PERMISSIONS`.
A task that turns out to be all safe-listed work still costs a restart to stop
the prompts, and a restart loses the conversation.

`yolo` also got safer lately: the deny list and the sudo gate now hold in every
mode, so the mode means "no prompts and no path jail" rather than "no refusals".

## What Changes

- `/yolo` toggles the permission mode for the running session only. It never
  writes the config file.
- Turning it off restores the mode the session launched with, so `strict` comes
  back as `strict`.
- One transcript line per switch, naming the deny list and the path jail.
- The path jail follows the mode, as it does at startup.
- The command table grows to nine, and `/help` lists `/yolo`.

## Capabilities

### Modified Capabilities

- `tui`: nine slash commands, with `/yolo` toggling the session mode.

## Impact

- `src/agent.rs`: a launch-mode field and a toggle.
- `src/tui.rs`, `src/main.rs`: the command in both front-ends.
- `src/help.rs`: one entry.
- `openspec/specs/tui/spec.md`: the command-list requirement.
