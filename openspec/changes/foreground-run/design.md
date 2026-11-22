# Design

## Approaches

- **Exit code only (chosen).** Reuses `run_attached`; no capture.
- **Capture through `script`.** The model would see output, but it costs a
  process and full-screen apps fill the log with drawing codes. Rejected by
  the user.

## Path

In the shell tool, `foreground: true` skips the allow and safe-list checks
and always asks unless the mode is `yolo`; the deny list still applies. It
then calls `hooks.run_attached(&in_dir(root, cmd))`, which already hands the
mouse and cooked mode over and takes them back. Line mode's hooks return
`None`; that maps to the refusal.

The tool schema gains `foreground` with a one-line description. The system
prompt's "Never run interactive commands" becomes "Run interactive commands
only with `foreground: true`; otherwise stdin is closed."

## Size

Expected: under 1 KB.
