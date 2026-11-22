# Proposal

## Why

Changing the permission mode or the agent takes a command and a picker. The
keys live inside `/help`, mixed with the commands.

## What Changes

- Shift+Tab cycles the permission mode: normal, strict, yolo, normal. It
  applies at once, mid-turn included.
- Tab on an empty draft cycles agent profiles. Mid-turn, the switch applies
  when the turn ends. With text in the draft, or one agent, Tab does nothing;
  in the command list it still completes.
- → alone accepts a suggestion; Tab no longer does.
- Ctrl+C stops instead of quitting: it declines an open prompt and stops
  the turn, stops a running turn, or clears the draft. Idle with an empty
  draft, it asks for a second Ctrl+C within two seconds. Ctrl+D still quits
  at once.
- The status line names the agent and the mode, and each switch writes one
  transcript line.
- `/keys` opens the keys card that `?` opens; `/help` drops its keys section
  and points to both.

## Keyboard and resources

Shift+Tab is `CSI Z`, sent by every common terminal. One more command,
`/keys`, taking the list to 24.

## Capabilities

### Modified Capabilities

- `tui`: mode and agent keys, `/keys`, and help without keys.
