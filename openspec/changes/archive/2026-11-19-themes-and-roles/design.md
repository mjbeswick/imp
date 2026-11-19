# Design

## Approaches

1. **Eleven named roles, RGB only.** Chosen: one table per theme, and the
   256-colour value is the nearest cube or grey-ramp entry, so a theme is
   eleven hex values and nothing else.
2. **Keep six roles, add themes.** Rejected: borders, durations, and status
   would still share one grey, which is the main visual problem.
3. **Theme files.** Deferred: a format and parser to maintain for a need no one
   has raised.

## Decisions

- `Theme { name, light, rgb: [[u8; 3]; 11] }`, indexed by a `Slot` order:
  text, user, muted, border, accent, tool, success, error, code, link, select.
- `tui::Role` gains `Border`, `Success`, `Code`, and `Link`. `Thinking` maps to
  `muted`, `Info` to `accent`, `Assistant` to `text`, `Add` to `success`,
  `Delete` to `error`.
- `Palette::select(text)` paints the `select` background behind `user` text for
  the highlighted picker row.
- Tool rows paint the leading `✓` in `success` and `✗` in `error`, bold, and the
  rest in the row's role.
- `to_256(rgb)` picks the closer of the nearest 6×6×6 cube entry and the
  nearest grey-ramp entry by squared distance.
- Background guess, only when no theme is saved: `COLORFGBG` ending in `7` or
  `15` means light, any other value dark. Otherwise the chat writes
  `ESC ] 11 ; ? BEL` and reads the reply with `VMIN=0, VTIME=1` for at most
  100 ms; `rgb:RRRR/GGGG/BBBB` with luminance above one half means light. No
  reply means dark. Line mode uses `COLORFGBG` only, since it reads cooked
  input. The answer is stored once for the process.
- Palettes come from the published Solarized, Gruvbox, Catppuccin, Tokyo Night,
  and Nord schemes, as previewed with the user.

## Tests

- Every theme defines eleven distinct-enough slots: `text` differs from its
  background guess, and `select` differs from `text`.
- `to_256` maps pure colours to their cube corners and greys to the ramp.
- `COLORFGBG` parsing: `15;0` dark, `0;15` light, junk dark.
- OSC 11 reply parsing for light and dark answers and a truncated reply.
- A colour frame paints the picker highlight with a background and `✓` in the
  success colour.
- With no saved theme and a light guess, `default-light` is active.
