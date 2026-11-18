# Spec Delta

## Purpose

Defines the terminal interface: what the user sees while a turn runs, which keys exist, what the status line reports, and how the terminal is left after any exit.

## MODIFIED Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, and Ctrl+J for a newline, Esc to cancel, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T to expand thinking, Ctrl+H to hide thinking, Ctrl+L to clear the view, and Ctrl+C twice to quit.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Quit guard
- **WHEN** the user presses Ctrl+C once
- **THEN** the status line asks for a second press, and no second press within two seconds returns to normal

#### Scenario: Start a second line
- **WHEN** the user types a line and presses Shift+Enter
- **THEN** the caret moves to a new row, nothing is sent, and the first line stays in the composer

#### Scenario: Terminals without modifier reporting
- **WHEN** the terminal never reports Shift+Enter separately
- **THEN** Alt+Enter and Ctrl+J still start a new line

#### Scenario: Cancel a running turn
- **WHEN** the user presses Esc while a turn is streaming or running a tool
- **THEN** the turn stops at its next check, the partial answer stays on screen, and the input line accepts text again

### Requirement: Line mode frames each turn

The system SHALL print a prompt only when stdin is a terminal, SHALL start model output on a fresh line, and SHALL NOT emit a prompt or escape sequences when piped.

#### Scenario: Interactive line mode
- **WHEN** the user types a message and presses Enter
- **THEN** the answer begins on the next line, never appended to the typed line

#### Scenario: Piped
- **WHEN** input is a pipe
- **THEN** output contains no `> ` prompt and no escape bytes

#### Scenario: Enter arrives as a bare carriage return
- **WHEN** the terminal sends `\r` without translating it to `\n`
- **THEN** the entry is submitted and no `^M` appears in the input

### Requirement: The terminal is always restored

The system SHALL restore cursor visibility, cooked input mode, and the primary screen on normal exit, panic, SIGINT, and SIGTERM.

#### Scenario: Forced panic
- **WHEN** the process panics while the alternate screen is active
- **THEN** the shell prompt returns usable, with no leftover alternate screen or hidden cursor

#### Scenario: Clean exit
- **WHEN** the user quits the TUI
- **THEN** the terminal input mode matches the mode from before the TUI started, and a line-mode program started afterwards submits on Enter

#### Scenario: Signal during a turn
- **WHEN** the process receives SIGINT or SIGTERM while a turn runs
- **THEN** the primary screen and cooked input mode are restored before it exits
