# Proposal

## Why

Line mode is plain text even on a colour terminal. Tool results are appended to
the exit line with one space, failures use `!` instead of the chat `✗`, and a
large job output is printed whole. `--chat` exists only to force the full screen
when output is piped, which no other mode needs. Appearance is hardcoded, so
there is no way to switch to a colour-blind-safe or low-contrast palette.

## What Changes

- Paint line-mode roles with the active palette: tool, error, info, and the
  prompt; no escape bytes when piped or when colour is off.
- Give each tool call a coloured start row and a `✓`/`✗` finish row with the
  duration, a preview capped at two lines or 240 characters, and failures on
  their own indented line.
- Remove `--chat`. Chat runs when stdin and stdout are terminals; `--cli`
  remains the only override.
- Add `/theme <name>` with four built-in themes, a bare form that lists them
  and marks the current one, and a state file so the choice survives restarts.
- Update `--help`, the README, and the command list when this lands. The list
  is deferred because `image-paste` and `session-fork` also change it.

## Capabilities

### Modified Capabilities

- `tui`: line mode uses the role palette and aligned tool rows; the front-end
  rule no longer has a forced-chat flag; named themes apply to both front-ends.

## Impact

- `src/tui.rs`: palette and theme types, role colours, the `/theme` command.
- `src/main.rs`: coloured printing, capped previews, `--chat` removal.
- `src/config.rs`: read the theme state at startup.
- `src/help.rs` and `README.md`.
- `openspec/specs/tui/spec.md`.
