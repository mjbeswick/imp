# Design

## Approaches

1. **SGR mouse reporting (`?1000h` with `?1006h`).** Chosen with the user:
   the wheel arrives as its own event, so it scrolls the transcript without
   touching history or the composer.
2. **Alternate scroll mode (`?1007h`).** Rejected: the wheel arrives as Up and
   Down, which recall history and move the caret, and support varies.
3. **Leave the mouse alone.** Rejected: that is the bug.

## Decisions

- A process-wide `MOUSE` flag is set from `cfg.mouse` before raw mode starts.
- `RawMode::enter` writes `?1000h ?1006h` after the alternate screen when the
  flag is on. `ALT_OFF` and the signal-safe `RESTORE` always write
  `?1006l ?1000l` first; turning off what was never on is harmless.
- `run_attached`, used by `/shell` and attached commands, turns reporting off
  before handing the terminal over and back on after.
- Decoding: `CSI < b ; x ; y M` or `m`. Button 64 is `Key::WheelUp`, 65 is
  `Key::WheelDown`, anything else is `Key::Unknown` and ignored.
- `WheelUp` scrolls the transcript up three rows and detaches; `WheelDown`
  scrolls down three and resumes following at the end. In a picker each step
  moves the highlight by one. Prompts ignore the wheel.
- `mouse` parses like other booleans and defaults to `on`.

## Tests

- Decoding wheel up, wheel down, and a click.
- Wheel up detaches and moves the view; wheel down at the end follows.
- Wheel down in a picker moves the highlight.
- `mouse=off` parses and the enable sequence depends on it.
