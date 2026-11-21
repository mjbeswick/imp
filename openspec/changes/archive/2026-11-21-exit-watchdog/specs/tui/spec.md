## MODIFIED Requirements

### Requirement: The terminal is always restored

The system SHALL restore cursor visibility, cooked input mode, and the primary screen on normal exit, panic, SIGINT, SIGTERM, and SIGHUP, and SHALL exit within about a second of such a signal even when the terminal has stopped reading.

#### Scenario: Forced panic
- **WHEN** the process panics while the alternate screen is active
- **THEN** the shell prompt returns usable, with no leftover alternate screen or hidden cursor

#### Scenario: Clean exit
- **WHEN** the user quits the TUI
- **THEN** the terminal input mode matches the mode from before the TUI started, and a line-mode program started afterwards submits on Enter

#### Scenario: Signal during a turn
- **WHEN** the process receives SIGINT or SIGTERM while a turn runs
- **THEN** the primary screen and cooked input mode are restored before it exits

#### Scenario: Linux terminal
- **WHEN** the interface runs on Linux
- **THEN** raw mode is applied with the platform's own `termios` layout, so typed keys reach the composer and the original mode is restored on exit

#### Scenario: Terminal not reading
- **WHEN** the terminal stops reading, as a frozen SSH session does, and the process receives SIGTERM or SIGHUP
- **THEN** it exits within about a second
