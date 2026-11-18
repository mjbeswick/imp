# Spec Delta

## MODIFIED Requirements

### Requirement: The interface is keyboard only

The system SHALL provide Enter to send, Shift+Enter, Alt+Enter, and Ctrl+J for a newline, Esc to cancel, Ctrl+R to retry, Ctrl+O to expand the last tool output, Ctrl+T to expand thinking, Ctrl+H to hide thinking, Ctrl+L to clear the view, and Ctrl+C to exit when idle, to exit from an open prompt other than a command confirmation, or to cancel a running turn and then exit on the next press.

#### Scenario: Send a message
- **WHEN** the user types text and presses Enter
- **THEN** the text is sent and the input clears

#### Scenario: Quit guard
- **WHEN** the session is idle and the user presses Ctrl+C
- **THEN** the app exits on that press

#### Scenario: Cancel then exit
- **WHEN** a turn is running and the user presses Ctrl+C
- **THEN** the turn is cancelled, the status row offers a second press to exit, and pressing Ctrl+C again exits even if the turn has not finished unwinding

#### Scenario: Picker open
- **WHEN** the session picker, agent picker, or an `ask` question is open and the user presses Ctrl+C
- **THEN** the app exits instead of swallowing the key

#### Scenario: Confirmation open
- **WHEN** a command confirmation is open and the user presses Ctrl+C
- **THEN** the command is refused and the turn stops, and the app keeps running

#### Scenario: Start a second line
- **WHEN** the user types a line and presses Shift+Enter
- **THEN** the caret moves to a new row, nothing is sent, and the first line stays in the composer

#### Scenario: Terminals without modifier reporting
- **WHEN** the terminal never reports Shift+Enter separately
- **THEN** Alt+Enter and Ctrl+J still start a new line

#### Scenario: Cancel a running turn
- **WHEN** the user presses Esc while a turn is streaming or running a tool
- **THEN** the turn stops at its next check, the partial answer stays on screen, and the input line accepts text again
