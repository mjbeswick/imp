# Proposal

## Why

Six colours cover the whole screen, so one grey paints thinking, rules, the
picker frame, the status line, durations, and the welcome row, and the picker
highlight has no background. Diff colours are fixed across themes. Every theme
is dark, so on a light terminal the default reply text is nearly invisible.

## What Changes

- Eleven roles: `text`, `user`, `muted`, `border`, `accent`, `tool`,
  `success`, `error`, `code`, `link`, and `select` (a background).
- `✓` marks and added lines use `success`; inline code uses `code`; links use
  `link`; rules and the picker frame use `border`; the highlighted picker row
  gets the `select` background.
- Eleven themes: `default`, `default-light`, `solarized-dark`,
  `solarized-light`, `gruvbox-dark`, `gruvbox-light`, `catppuccin-mocha`,
  `catppuccin-latte`, `tokyo-night`, `nord`, and `mono`.
- 256-colour fallbacks are computed from each RGB value instead of listed.
- With no saved theme, imp picks `default` or `default-light` from the
  terminal background: `COLORFGBG` first, then an OSC 11 query with a 100 ms
  limit in the chat, then dark.
- `mono` keeps forcing colour off and carries no colours.

## Out of Scope

- User theme files.
- Dracula and One Dark.

## Capabilities

### Modified Capabilities

- `tui`: roles, themes, and the background guess.

## Impact

- `src/theme.rs`, `src/tui.rs`, `README.md`.
