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
mouse and cooked mode over and takes them back. Line mode runs privileged
commands through `run_attached` too, so a new hook, `can_foreground`, true
only in the chat, decides the refusal.

The tool schema gains `foreground` with a one-line description. The system
prompt's "Never run interactive commands" becomes "Run interactive commands
only with `foreground: true`; otherwise stdin is closed."

## Size

Expected: under 1 KB.

## Owning the terminal

Found while testing `/shell`, which shares the attached path with password
prompts and this change: the key reader kept reading while the command held
the terminal. From the background that read stopped imp, so the user's own
shell regained the terminal and ended; under tmux, where imp's group is
orphaned, it failed instead and the reader thread exited, leaving imp deaf.
imp also kept drawing frames over the command.

The reader now waits in `poll` and never reads while `ATTACHED` is set.
`run_attached` sets it, waits for the reader to report `PARKED`, and clears
it once the terminal is back. The main loop draws nothing while attached,
wakes every 50 ms to see the return, and then repaints every row.
