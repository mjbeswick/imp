# Proposal

## Why

`yolo` reads as "no prompts", but three paths still ask: writes to imp's own
config, `sudo` when `allow_sudo=true`, and memory saves, including facts
offered at compaction. Nothing on screen shows that the mode is on, either.

## What Changes

- In `yolo`, no permission prompt appears. The deny list still refuses, and the
  model's `ask` tool still asks questions.
- imp's own config file: refused in `normal` and `strict`, allowed without a
  prompt in `yolo`.
- `sudo` with `allow_sudo=true` runs without a prompt in `yolo`.
- Memory saves and compaction facts save without a prompt in `yolo`.
- In `yolo`, the composer prompt glyph and the rule above it use the error
  colour, in the chat and in line mode. The status line text is unchanged.

## Capabilities

### Modified Capabilities

- `permissions`: `yolo` never prompts; own-config rule.
- `memory`: compaction facts in `yolo`.
- `tui`: `yolo` colour cue.

## Impact

- `src/perm.rs`, `src/agent.rs`, `src/tui.rs`, `src/main.rs`.
