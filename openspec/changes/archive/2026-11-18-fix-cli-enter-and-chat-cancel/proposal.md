# Proposal

## Why

Three input defects share one cause: imp changes the terminal mode and then
assumes it is unchanged.

- `imp --cli` waits for a line feed. When the terminal's carriage-return
  mapping is off, Enter sends a bare carriage return, so the prompt shows `^M`
  and never submits.
- The TUI leaves raw input mode on for some exits. The shell that started it
  stays broken, and every later line-mode program inherits the damage.
- Esc during a running turn only sets a flag inside the worker's queue. The
  worker cannot read it until the turn returns, so the turn keeps running.

## What Changes

- Line mode reads an entry byte by byte, ending on carriage return or line feed.
- Raw mode is set and restored with termios directly, on normal exit, panic,
  SIGINT, and SIGTERM.
- Esc writes the session cancel flag directly, so the running turn stops.
- Tests cover all three.

## Capabilities

### Modified Capabilities

- `tui`: line mode accepts either Enter encoding, the terminal is always
  restored, and Esc stops a running turn.

## Impact

- `src/tui.rs`: raw mode, restore paths, Esc handling.
- `src/main.rs`: line-mode entry reader.
- `src/agent.rs`: cancel handle accessor.
- `tests/tty.rs`: new terminal-restore test.
- `openspec/specs/tui/spec.md`: three requirements restated.
- No dependency, configuration, or data changes.
