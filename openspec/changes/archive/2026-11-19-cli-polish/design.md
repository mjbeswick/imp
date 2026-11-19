# Design

## Approaches

1. **A theme table in a small module, plus one state file.** Chosen: colours
   stay in code, the choice stays in state, and no commented config is
   rewritten.
2. **Theme files under the config directory.** More flexible, but needs a file
   format, parsing, and validation for four built-ins.
3. **Rewrite the config's `theme=` line.** Fights the commented file and the
   rule that the agent does not edit its own config.

## Decisions

- New `src/theme.rs` holds `Theme { name, roles }`. Each role carries an RGB
  triple and a 256-colour fallback. Roles are user, assistant, thinking, tool,
  error, info, add, delete, and border, so the diff and picker changes do not
  need to revisit every palette.
- Themes: `default` keeps today's colours, `mono` forces colour off,
  `solarized-dark` and `gruvbox-dark` provide the two named palettes.
- The choice lives in `<state>/imp/theme`, one plain name. Startup reads it;
  an unknown name falls back to `default` and says nothing unless the user
  opens `/theme`.
- `Palette` keeps `on`, `truecolor`, and the active theme. `mono` sets `on` to
  false even when the terminal supports colour. `NO_COLOR` and `color=never`
  still win.
- `/theme <name>` updates the palette in chat, saves the state file, and
  reports the name. Bare `/theme` lists the four names with the current one
  marked. An unknown name lists them and changes nothing.
- Line mode builds the same `Palette` at startup, paints the prompt, tool
  rows, errors, and info lines, and keeps plain output when piped.
- `print_ui_to` gains the palette and caps previews with one helper:
  `preview(text) = first 2 lines or 240 chars + "… (+N more)"`.
- Tool start prints `▸ summary`; tool end prints `  ✓|✗ {ms} ms`. The end row
  never repeats the summary, so a `du` result cannot flood the screen.
- `--chat` is removed from the parser and help. `wants_chat(mode, tty)` returns
  false for `cli` and otherwise `tty`. The test that expected `chat` is
  removed, and `--chat` gets an unknown-option test.
- The slash-command list delta is deferred until this change lands, after
  `image-paste` and `session-fork`, because all three alter the list.
- Tests cover theme load/save/fallback, mono, list and unknown names, colour
  versus piped line output, preview caps, spacing, and `--chat` rejection.
